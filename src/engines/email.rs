use anyhow::{bail, Result};
use console::style;
use hickory_resolver::TokioAsyncResolver;
use md5::{Digest, Md5};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::confidence::{Confidence, Finding};
use crate::config::AppConfig;
use crate::storage::Store;

fn gravatar_hash(email: &str) -> String {
    let mut h = Md5::new();
    h.update(email.trim().to_lowercase().as_bytes());
    format!("{:x}", h.finalize())
}

pub async fn scan(email: &str, strict: bool) -> Result<Vec<Finding>> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 5 {
        bail!("invalid email");
    }
    println!("{}", style(format!("Email engine · {email}")).cyan().bold());

    let mut findings = Vec::new();
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Aliascope/0.3 (+legal-osint)")
        .build()?;

    // Format + domain
    let domain = email.split('@').nth(1).unwrap_or("");
    findings.push(Finding {
        engine: "email".into(),
        subject: email.clone(),
        title: "Format".into(),
        detail: "contains @ and local part".into(),
        url: None,
        confidence: Confidence::High,
        reason: "RFC-like structural check".into(),
    });

    // DNS MX / A
    if let Ok(resolver) = TokioAsyncResolver::tokio_from_system_conf() {
        match resolver.mx_lookup(domain).await {
            Ok(mx) => {
                let records: Vec<String> = mx.iter().map(|r| r.exchange().to_string()).collect();
                findings.push(Finding {
                    engine: "dns".into(),
                    subject: domain.into(),
                    title: "MX records".into(),
                    detail: records.join(", "),
                    url: None,
                    confidence: Confidence::High,
                    reason: "Authoritative-style MX lookup via system resolver".into(),
                });
            }
            Err(_) => {
                findings.push(Finding {
                    engine: "dns".into(),
                    subject: domain.into(),
                    title: "MX records".into(),
                    detail: "no MX / lookup failed".into(),
                    url: None,
                    confidence: Confidence::Medium,
                    reason: "Domain may not accept mail".into(),
                });
            }
        }
        if let Ok(a) = resolver.ipv4_lookup(domain).await {
            let ips: Vec<String> = a.iter().map(|i| i.to_string()).collect();
            findings.push(Finding {
                engine: "dns".into(),
                subject: domain.into(),
                title: "A records".into(),
                detail: ips.join(", "),
                url: None,
                confidence: Confidence::High,
                reason: "DNS A lookup".into(),
            });
        }
    }

    // Gravatar
    let gh = gravatar_hash(&email);
    let gurl = format!("https://www.gravatar.com/avatar/{gh}?d=404");
    if let Ok(resp) = client.get(&gurl).send().await {
        if resp.status().as_u16() == 200 {
            findings.push(Finding {
                engine: "gravatar".into(),
                subject: email.clone(),
                title: "Public Gravatar".into(),
                detail: format!("hash {gh}"),
                url: Some(gurl),
                confidence: Confidence::High,
                reason: "Gravatar returned image for email hash".into(),
            });
        }
    }

    // Firefox accounts status
    let fxa = format!(
        "https://api.accounts.firefox.com/v1/account/status?email={}",
        urlencoding::encode(&email)
    );
    if let Ok(resp) = client.get(&fxa).send().await {
        if let Ok(v) = resp.json::<Value>().await {
            if let Some(exists) = v.get("exists").and_then(|x| x.as_bool()) {
                findings.push(Finding {
                    engine: "firefox".into(),
                    subject: email.clone(),
                    title: "Firefox Accounts".into(),
                    detail: format!("exists={exists}"),
                    url: None,
                    confidence: Confidence::High,
                    reason: "Public FXA status API".into(),
                });
            }
        }
    }

    // Manual HIBP link
    findings.push(Finding {
        engine: "hibp_manual".into(),
        subject: email.clone(),
        title: "Manual breach check".into(),
        detail: "Open free website check".into(),
        url: Some(format!("https://haveibeenpwned.com/account/{email}")),
        confidence: Confidence::Info,
        reason: "No password data; user-operated check".into(),
    });

    // Optional HIBP API
    let cfg = AppConfig::load().unwrap_or_default();
    if !cfg.hibp_api_key.trim().is_empty() {
        let url = format!(
            "https://haveibeenpwned.com/api/v3/breachedaccount/{}?truncateResponse=true",
            urlencoding::encode(&email)
        );
        if let Ok(resp) = client
            .get(url)
            .header("hibp-api-key", cfg.hibp_api_key.trim())
            .header("user-agent", "Aliascope/0.3")
            .send()
            .await
        {
            match resp.status().as_u16() {
                200 => {
                    let body: Value = resp.json().await.unwrap_or(Value::Null);
                    let n = body.as_array().map(|a| a.len()).unwrap_or(0);
                    findings.push(Finding {
                        engine: "hibp".into(),
                        subject: email.clone(),
                        title: "Breach presence".into(),
                        detail: format!("{n} breach name(s) — passwords not returned"),
                        url: None,
                        confidence: Confidence::High,
                        reason: "Official HIBP API".into(),
                    });
                }
                404 => findings.push(Finding {
                    engine: "hibp".into(),
                    subject: email.clone(),
                    title: "Breach presence".into(),
                    detail: "not found in HIBP dataset".into(),
                    url: None,
                    confidence: Confidence::Medium,
                    reason: "Official HIBP API 404".into(),
                }),
                _ => {}
            }
        }
    }

    if strict {
        findings.retain(|f| f.passes_strict());
    }

    if let Ok(st) = Store::open_default() {
        let _ = st.save_scan("email", &email, &findings);
    }

    for f in &findings {
        println!(
            "  [{}] {} — {} ({})",
            f.confidence.as_str(),
            style(&f.title).bold(),
            f.detail,
            f.reason
        );
    }
    Ok(findings)
}
