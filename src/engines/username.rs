use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use crate::ai;
use crate::confidence::{Confidence, Finding};
use crate::config::{AppConfig, Settings};
use crate::platforms;
use crate::queue;
use crate::storage::Store;

pub fn generate_aliases(base: &str, limit: usize) -> Vec<String> {
    let cleaned = base.trim().to_lowercase();
    let re = Regex::new(r"[^a-z0-9._-]").unwrap();
    let core = re.replace_all(&cleaned, "").to_string();
    if core.is_empty() {
        return vec![];
    }
    let mut set = BTreeSet::new();
    set.insert(core.clone());
    for s in ["1", "01", "123", "official", "real", "dev", "hq", "x", "io", "tech"] {
        set.insert(format!("{core}{s}"));
        set.insert(format!("{core}_{s}"));
        set.insert(format!("{core}.{s}"));
    }
    for y in ["2022", "2023", "2024", "2025", "2026"] {
        set.insert(format!("{core}{y}"));
        set.insert(format!("{core}_{y}"));
    }
    for p in ["the", "its", "im"] {
        set.insert(format!("{p}{core}"));
        set.insert(format!("{p}_{core}"));
    }
    let leet = core.replace('a', "4").replace('e', "3").replace('i', "1").replace('o', "0");
    if leet != core {
        set.insert(leet);
    }
    set.into_iter().take(limit.max(5)).collect()
}

fn classify_status(status: u16) -> (bool, Confidence, &'static str) {
    match status {
        200 => (true, Confidence::Medium, "HTTP 200 public page"),
        301 | 302 | 303 => (true, Confidence::Low, "HTTP redirect — possible profile or login"),
        404 => (false, Confidence::Info, "not found"),
        0 => (false, Confidence::Info, "network error"),
        _ => (false, Confidence::Low, "non-success status"),
    }
}

pub async fn scan(query: &str, limit: usize, strict: bool) -> Result<Vec<Finding>> {
    let settings = Settings::load();
    let cfg = AppConfig::load().unwrap_or_default();
    let store = Store::open_default().ok();

    println!("{}", style(format!("Username engine · «{query}»")).cyan().bold());

    let mut aliases = generate_aliases(query, limit);
    if let Ok(extra) = ai::suggest_aliases(query).await {
        for a in extra {
            if !aliases.iter().any(|x| x == &a) {
                aliases.push(a);
            }
        }
        aliases.truncate(limit + 15);
    }

    // similarity keep near seed
    aliases.retain(|a| {
        let s = strsim::jaro_winkler(query, a);
        s >= 0.55 || a.contains(&query.to_lowercase())
    });

    let platforms = platforms::load_platforms()?;
    println!(
        "{} {} aliases × {} platforms (concurrency {})",
        style("→").green(),
        aliases.len(),
        platforms.len(),
        settings.max_concurrent
    );

    let mut builder = Client::builder()
        .timeout(Duration::from_secs(settings.request_timeout_sec))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent(settings.user_agent.clone());
    if let Some(proxy) = &settings.proxy {
        if let Ok(p) = reqwest::Proxy::all(proxy) {
            builder = builder.proxy(p);
        }
    }
    let client = Arc::new(builder.build()?);

    let mut jobs = Vec::new();
    for alias in &aliases {
        for p in &platforms {
            jobs.push((alias.clone(), p.name.clone(), platforms::profile_url(&p.url, alias)));
        }
    }

    let pb = ProgressBar::new(jobs.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len}")?
            .progress_chars("=>-"),
    );

    let rate = settings.rate_limit_ms;
    let ttl = settings.cache_ttl_hours;
    let ua_token = cfg.github_token.clone();

    let results = queue::map_limited(jobs, settings.max_concurrent, {
        let client = Arc::clone(&client);
        let store_path = dirs::home_dir().map(|h| h.join(".aliascope/aliascope.db"));
        move |(alias, platform, url)| {
            let client = Arc::clone(&client);
            let store_path = store_path.clone();
            let token = ua_token.clone();
            let pb = pb.clone();
            async move {
                // cache
                if let Some(path) = &store_path {
                    if let Ok(st) = Store::open(path.clone()) {
                        if let Ok(Some(stt)) = st.cache_get(&url, ttl) {
                            pb.inc(1);
                            let (keep, conf, reason) = classify_status(stt);
                            return (keep, alias, platform, url, stt, conf, reason);
                        }
                    }
                }

                let mut req = client.get(&url);
                if platform == "GitHub" && !token.is_empty() {
                    // only for api hosts ideally; harmless on html
                    req = req.header("Authorization", format!("Bearer {token}"));
                }
                let status = match req.send().await {
                    Ok(r) => r.status().as_u16(),
                    Err(_) => 0,
                };
                if let Some(path) = &store_path {
                    if let Ok(st) = Store::open(path.clone()) {
                        let _ = st.cache_put(&url, status);
                    }
                }
                tokio::time::sleep(Duration::from_millis(rate)).await;
                pb.inc(1);
                let (keep, conf, reason) = classify_status(status);
                (keep, alias, platform, url, status, conf, reason)
            }
        }
    })
    .await;
    pb.finish_with_message("done");

    // Gravatar cross-link signal for email-like? skip for pure username

    // GitHub API public user if token/public
    let mut findings = Vec::new();
    if let Ok(Some(f)) = github_public_user(&client, query, &cfg.github_token).await {
        findings.push(f);
    }

    for (keep, alias, platform, url, status, conf, reason) in results {
        if !keep {
            continue;
        }
        if strict && conf != Confidence::High {
            continue;
        }
        findings.push(Finding {
            engine: "username".into(),
            subject: alias.clone(),
            title: format!("{platform} profile signal"),
            detail: format!("HTTP {status}"),
            url: Some(url),
            confidence: conf,
            reason: reason.into(),
        });
    }

    // Optional AI triage
    if let Ok(summary) = ai::triage_findings(query, &findings).await {
        findings.push(Finding {
            engine: "ai".into(),
            subject: query.into(),
            title: "Analyst summary".into(),
            detail: summary,
            url: None,
            confidence: Confidence::Info,
            reason: "Model summary over observed findings only".into(),
        });
    }

    if let Some(st) = store {
        let _ = st.save_scan("user", query, &findings);
    }

    print_findings(&findings);
    Ok(findings)
}

async fn github_public_user(
    client: &Client,
    login: &str,
    token: &str,
) -> Result<Option<Finding>> {
    let url = format!("https://api.github.com/users/{}", urlencoding::encode(login));
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Aliascope/0.3");
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    let resp = req.send().await?;
    if resp.status().as_u16() != 200 {
        return Ok(None);
    }
    let v: serde_json::Value = resp.json().await?;
    let html = v.get("html_url").and_then(|x| x.as_str()).unwrap_or("");
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
    let bio = v.get("bio").and_then(|x| x.as_str()).unwrap_or("");
    Ok(Some(Finding {
        engine: "github_api".into(),
        subject: login.into(),
        title: "GitHub API user".into(),
        detail: format!("name={name} bio={}", bio.chars().take(80).collect::<String>()),
        url: Some(html.into()),
        confidence: Confidence::High,
        reason: "Official GitHub public API 200".into(),
    }))
}

fn print_findings(findings: &[Finding]) {
    println!();
    if findings.is_empty() {
        println!("{}", style("No findings.").yellow());
        return;
    }
    println!("{}", style(format!("Findings ({})", findings.len())).green().bold());
    for f in findings {
        println!(
            "  [{}] {} · {} — {} ({})",
            f.confidence.as_str(),
            style(&f.engine).bold(),
            f.title,
            f.detail,
            f.reason
        );
        if let Some(u) = &f.url {
            println!("      {u}");
        }
    }
}
