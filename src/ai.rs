use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use crate::config::AppConfig;
use crate::username::Hit;

fn client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(45))
        .build()?)
}

/// Agent A — expand username ideas (OpenAI-compatible primary key).
pub async fn suggest_aliases(seed: &str) -> Result<Vec<String>> {
    let cfg = AppConfig::load()?;
    if cfg.ai_key_primary.trim().is_empty() {
        bail!("primary AI key not set");
    }

    let body = json!({
        "model": "llama-3.1-8b-instant",
        "temperature": 0.4,
        "messages": [
            {
                "role": "system",
                "content": "You generate plausible public social-media username variants. Reply with a pure JSON array of strings only. No passwords, no private data."
            },
            {
                "role": "user",
                "content": format!(
                    "Seed handle or name: {seed}. Return up to 12 username candidates that a real person might register (ascii, no spaces)."
                )
            }
        ]
    });

    let url = format!(
        "{}/chat/completions",
        cfg.ai_primary_base.trim_end_matches('/')
    );
    let resp = client()?
        .post(url)
        .bearer_auth(cfg.ai_key_primary.trim())
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        bail!("primary AI HTTP {}", resp.status());
    }
    let v: Value = resp.json().await?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("[]");
    let cleaned = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let arr: Vec<String> = serde_json::from_str(cleaned).unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|s| {
            s.to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '-')
                .collect()
        })
        .filter(|s: &String| s.len() >= 2)
        .collect())
}

/// Agent B — summarize public hits (Gemini-style secondary if key present; falls back to primary).
pub async fn summarize_hits(seed: &str, hits: &[Hit]) -> Result<String> {
    let cfg = AppConfig::load()?;
    let lines: Vec<String> = hits
        .iter()
        .take(40)
        .map(|h| format!("{} | {} | {}", h.platform, h.alias, h.url))
        .collect();
    let prompt = format!(
        "Seed: {seed}\nPublic probe hits (platform | alias | url):\n{}\n\nWrite a short neutral OSINT-style summary in English: clusters of aliases, platforms with most hits, and caution that HTTP presence is not identity proof. No passwords. No accusations.",
        lines.join("\n")
    );

    // Prefer secondary Gemini REST if key set
    if !cfg.ai_key_secondary.trim().is_empty() {
        if let Ok(text) = gemini_generate(&cfg, &prompt).await {
            return Ok(text);
        }
    }
    if cfg.ai_key_primary.trim().is_empty() {
        bail!("no AI keys configured");
    }

    let body = json!({
        "model": "llama-3.1-8b-instant",
        "temperature": 0.2,
        "messages": [
            {"role": "system", "content": "You are a careful OSINT analyst. Never invent private data or passwords."},
            {"role": "user", "content": prompt}
        ]
    });
    let url = format!(
        "{}/chat/completions",
        cfg.ai_primary_base.trim_end_matches('/')
    );
    let resp = client()?
        .post(url)
        .bearer_auth(cfg.ai_key_primary.trim())
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("summary AI HTTP {}", resp.status());
    }
    let v: Value = resp.json().await?;
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("(empty)")
        .to_string())
}

async fn gemini_generate(cfg: &AppConfig, prompt: &str) -> Result<String> {
    let base = cfg.ai_secondary_base.trim_end_matches('/');
    let url = format!(
        "{}/models/gemini-1.5-flash:generateContent?key={}",
        base,
        urlencoding::encode(cfg.ai_key_secondary.trim())
    );
    let body = json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }]
    });
    let resp = client()?.post(url).json(&body).send().await?;
    if !resp.status().is_success() {
        bail!("gemini HTTP {}", resp.status());
    }
    let v: Value = resp.json().await?;
    let text = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if text.is_empty() {
        bail!("empty gemini");
    }
    Ok(text)
}
