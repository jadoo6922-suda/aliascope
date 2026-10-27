use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use crate::confidence::Finding;
use crate::config::AppConfig;

fn client() -> Result<Client> {
    Ok(Client::builder().timeout(Duration::from_secs(45)).build()?)
}

pub async fn suggest_aliases(seed: &str) -> Result<Vec<String>> {
    let cfg = AppConfig::load()?;
    if cfg.ai_key_primary.trim().is_empty() {
        bail!("no primary key");
    }
    let body = json!({
        "model": "llama-3.1-8b-instant",
        "temperature": 0.4,
        "messages": [
            {"role":"system","content":"Generate plausible public usernames. JSON array of strings only. No passwords."},
            {"role":"user","content": format!("Seed: {seed}. Up to 12 ascii usernames.")}
        ]
    });
    let url = format!("{}/chat/completions", cfg.ai_primary_base.trim_end_matches('/'));
    let resp = client()?
        .post(url)
        .bearer_auth(cfg.ai_key_primary.trim())
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("ai http");
    }
    let v: Value = resp.json().await?;
    let content = v["choices"][0]["message"]["content"].as_str().unwrap_or("[]");
    let cleaned = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let arr: Vec<String> = serde_json::from_str(cleaned).unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|s| {
            s.to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
                .collect()
        })
        .filter(|s: &String| s.len() >= 2)
        .collect())
}

/// Analyst only — must not invent accounts; summarize observed findings.
pub async fn triage_findings(seed: &str, findings: &[Finding]) -> Result<String> {
    let cfg = AppConfig::load()?;
    if cfg.ai_key_primary.trim().is_empty() && cfg.ai_key_secondary.trim().is_empty() {
        bail!("no ai keys");
    }
    let lines: Vec<String> = findings
        .iter()
        .filter(|f| f.engine != "ai")
        .take(40)
        .map(|f| {
            format!(
                "[{}] {} | {} | {} | {}",
                f.confidence.as_str(),
                f.engine,
                f.title,
                f.detail,
                f.url.as_deref().unwrap_or("-")
            )
        })
        .collect();
    if lines.is_empty() {
        bail!("no findings");
    }
    let prompt = format!(
        "Seed: {seed}\nObserved findings only:\n{}\n\nRespond in English with sections: Confirmed (high), Possible (medium), Weak (low). Suggest manual verification steps. Do NOT invent accounts or passwords. Every claim must reference an observed line.",
        lines.join("\n")
    );

    if !cfg.ai_key_primary.trim().is_empty() {
        let body = json!({
            "model": "llama-3.1-8b-instant",
            "temperature": 0.2,
            "messages": [
                {"role":"system","content":"Careful OSINT analyst. Never invent private data."},
                {"role":"user","content": prompt}
            ]
        });
        let url = format!("{}/chat/completions", cfg.ai_primary_base.trim_end_matches('/'));
        let resp = client()?
            .post(url)
            .bearer_auth(cfg.ai_key_primary.trim())
            .json(&body)
            .send()
            .await?;
        if resp.status().is_success() {
            let v: Value = resp.json().await?;
            if let Some(t) = v["choices"][0]["message"]["content"].as_str() {
                return Ok(t.to_string());
            }
        }
    }
    bail!("ai failed")
}
