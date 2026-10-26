use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use std::collections::BTreeSet;
use std::time::Duration;

use crate::ai;
use crate::platforms;

#[derive(Debug)]
pub struct Hit {
    pub alias: String,
    pub platform: String,
    pub url: String,
    pub status: u16,
    pub likely_exists: bool,
}

pub fn generate_aliases(base: &str, limit: usize) -> Vec<String> {
    let cleaned = base.trim().to_lowercase();
    let re = Regex::new(r"[^a-z0-9._-]").unwrap();
    let core = re.replace_all(&cleaned, "").to_string();
    if core.is_empty() {
        return vec![];
    }

    let mut set = BTreeSet::new();
    set.insert(core.clone());

    let suffixes = [
        "1", "01", "123", "007", "x", "xx", "official", "real", "dev", "hq",
        "team", "app", "io", "tech", "sys", "ops",
    ];
    let years = ["2020", "2021", "2022", "2023", "2024", "2025", "2026"];
    let prefixes = ["the", "its", "im", "mr", "ms"];

    for s in suffixes {
        set.insert(format!("{core}{s}"));
        set.insert(format!("{core}_{s}"));
        set.insert(format!("{core}.{s}"));
        set.insert(format!("{core}-{s}"));
    }
    for y in years {
        set.insert(format!("{core}{y}"));
        set.insert(format!("{core}_{y}"));
    }
    for p in prefixes {
        set.insert(format!("{p}{core}"));
        set.insert(format!("{p}_{core}"));
        set.insert(format!("{p}.{core}"));
    }

    // light leetspeak
    let leet = core
        .replace('a', "4")
        .replace('e', "3")
        .replace('i', "1")
        .replace('o', "0")
        .replace('s', "5");
    if leet != core {
        set.insert(leet);
    }

    set.into_iter().take(limit.max(5)).collect()
}

async fn probe_url(client: &Client, url: &str) -> (u16, bool) {
    match client
        .get(url)
        .header(
            "User-Agent",
            "Aliascope/0.1 (+https://github.com/aliascope; legal-osint; respectful)",
        )
        .timeout(Duration::from_secs(12))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            // Conservative: 200 often means public page exists; 404 absent.
            // 301/302 may be profile or login redirect — mark as possible.
            let likely = matches!(status, 200 | 301 | 302 | 303);
            (status, likely)
        }
        Err(_) => (0, false),
    }
}

pub async fn run_user_scan(query: &str, limit: usize) -> Result<()> {
    println!(
        "{}",
        style(format!("Alias discovery for «{query}»")).cyan().bold()
    );
    println!(
        "{}",
        style("Probing public profile URLs only. Rate-limited. No login bypass.")
            .dim()
    );

    let mut aliases = generate_aliases(query, limit);

    // Optional AI expansion (if keys configured)
    if let Ok(extra) = ai::suggest_aliases(query).await {
        for a in extra {
            if !aliases.iter().any(|x| x == &a) {
                aliases.push(a);
            }
        }
        aliases.truncate(limit + 15);
    }

    println!(
        "{} {} candidates · {} platforms",
        style("→").green(),
        aliases.len(),
        platforms::all_platforms().len()
    );

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let platforms = platforms::all_platforms();
    let total = (aliases.len() * platforms.len()) as u64;
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )?
        .progress_chars("=>-"),
    );

    let mut hits: Vec<Hit> = Vec::new();

    for alias in &aliases {
        for p in &platforms {
            let url = p.profile_url.replace("{}", alias);
            pb.set_message(format!("{} @ {}", p.name, alias));
            let (status, likely) = probe_url(&client, &url).await;
            if likely {
                hits.push(Hit {
                    alias: alias.clone(),
                    platform: p.name.to_string(),
                    url: url.clone(),
                    status,
                    likely_exists: true,
                });
            }
            pb.inc(1);
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
    }
    pb.finish_with_message("scan complete");

    println!();
    if hits.is_empty() {
        println!("{}", style("No likely public profiles found.").yellow());
        return Ok(());
    }

    println!(
        "{}",
        style(format!("Likely public hits ({})", hits.len()))
            .green()
            .bold()
    );
    for h in &hits {
        println!(
            "  {} {}  {}  [{}]",
            style("•").green(),
            style(&h.platform).bold(),
            h.url,
            h.status
        );
    }

    // AI summary if available
    if let Ok(summary) = ai::summarize_hits(query, &hits).await {
        println!();
        println!("{}", style("AI summary").magenta().bold());
        println!("{summary}");
    }

    Ok(())
}
