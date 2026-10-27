use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use crate::ai;
use crate::export;
use crate::platforms;
use crate::storage::Store;

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub alias: String,
    pub platform: String,
    pub url: String,
    pub status: u16,
    pub likely_exists: bool,
}

pub struct ScanOptions {
    pub limit: usize,
    pub json_out: Option<PathBuf>,
    pub html_out: Option<PathBuf>,
    pub save_db: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            limit: 40,
            json_out: None,
            html_out: None,
            save_db: true,
        }
    }
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
        "1", "01", "123", "007", "x", "xx", "official", "real", "dev", "hq", "team", "app", "io",
        "tech", "sys", "ops",
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
            "Aliascope/0.2 (+https://github.com/jadoo6922-suda/aliascope; legal-osint)",
        )
        .timeout(Duration::from_secs(12))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let likely = matches!(status, 200 | 301 | 302 | 303);
            (status, likely)
        }
        Err(_) => (0, false),
    }
}

pub async fn run_user_scan(query: &str, opts: ScanOptions) -> Result<Vec<Hit>> {
    println!(
        "{}",
        style(format!("Alias discovery for «{query}»")).cyan().bold()
    );
    println!(
        "{}",
        style("Probing public profile URLs only. Rate-limited. No login bypass.").dim()
    );

    let mut aliases = generate_aliases(query, opts.limit);

    if let Ok(extra) = ai::suggest_aliases(query).await {
        for a in extra {
            if !aliases.iter().any(|x| x == &a) {
                aliases.push(a);
            }
        }
        aliases.truncate(opts.limit + 15);
    }

    let plist = platforms::load_platforms()?;
    println!(
        "{} {} candidates · {} platforms",
        style("→").green(),
        aliases.len(),
        plist.len()
    );

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let total = (aliases.len() * plist.len()) as u64;
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")?
            .progress_chars("=>-"),
    );

    let mut hits: Vec<Hit> = Vec::new();

    for alias in &aliases {
        for p in &plist {
            let url = platforms::profile_url(&p.url, alias);
            pb.set_message(format!("{} @ {}", p.name, alias));
            let (status, likely) = probe_url(&client, &url).await;
            if likely {
                hits.push(Hit {
                    alias: alias.clone(),
                    platform: p.name.clone(),
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
    } else {
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
    }

    if opts.save_db {
        match Store::open_default() {
            Ok(store) => match store.save_user_scan(query, &hits) {
                Ok(id) => println!(
                    "{} saved to local DB as scan #{id} (~/.aliascope/aliascope.db)",
                    style("✓").green()
                ),
                Err(e) => println!("{} DB save failed: {e}", style("!").yellow()),
            },
            Err(e) => println!("{} DB open failed: {e}", style("!").yellow()),
        }
    }

    if let Some(path) = &opts.json_out {
        export::write_json(path, query, &hits)?;
        println!("{} JSON report → {}", style("✓").green(), path.display());
    }
    if let Some(path) = &opts.html_out {
        export::write_html(path, query, &hits)?;
        println!("{} HTML report → {}", style("✓").green(), path.display());
    }

    if let Ok(summary) = ai::summarize_hits(query, &hits).await {
        println!();
        println!("{}", style("AI summary").magenta().bold());
        println!("{summary}");
    }

    Ok(hits)
}
