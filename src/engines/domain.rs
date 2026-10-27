use anyhow::{bail, Result};
use console::style;
use hickory_resolver::TokioAsyncResolver;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::confidence::{Confidence, Finding};
use crate::storage::Store;

pub async fn scan(domain: &str, strict: bool) -> Result<Vec<Finding>> {
    let domain = domain
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase();
    if domain.is_empty() || !domain.contains('.') {
        bail!("invalid domain");
    }
    println!("{}", style(format!("Domain engine · {domain}")).cyan().bold());
    let mut findings = Vec::new();

    if let Ok(resolver) = TokioAsyncResolver::tokio_from_system_conf() {
        if let Ok(a) = resolver.ipv4_lookup(domain.as_str()).await {
            findings.push(Finding {
                engine: "dns".into(),
                subject: domain.clone(),
                title: "A records".into(),
                detail: a.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "),
                url: None,
                confidence: Confidence::High,
                reason: "DNS A".into(),
            });
        }
        if let Ok(mx) = resolver.mx_lookup(domain.as_str()).await {
            findings.push(Finding {
                engine: "dns".into(),
                subject: domain.clone(),
                title: "MX records".into(),
                detail: mx
                    .iter()
                    .map(|x| x.exchange().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                url: None,
                confidence: Confidence::High,
                reason: "DNS MX".into(),
            });
        }
        if let Ok(txt) = resolver.txt_lookup(domain.as_str()).await {
            let joined = txt
                .iter()
                .map(|r| r.to_string())
                .take(8)
                .collect::<Vec<_>>()
                .join(" | ");
            if !joined.is_empty() {
                findings.push(Finding {
                    engine: "dns".into(),
                    subject: domain.clone(),
                    title: "TXT records".into(),
                    detail: joined.chars().take(300).collect(),
                    url: None,
                    confidence: Confidence::Medium,
                    reason: "DNS TXT".into(),
                });
            }
        }
    }

    // crt.sh certificate transparency (public)
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Aliascope/0.3")
        .build()?;
    let crt = format!(
        "https://crt.sh/?q={}&output=json",
        urlencoding::encode(&format!("%.{domain}"))
    );
    if let Ok(resp) = client.get(&crt).send().await {
        if resp.status().is_success() {
            if let Ok(arr) = resp.json::<Vec<Value>>().await {
                let mut names = std::collections::BTreeSet::new();
                for row in arr.iter().take(200) {
                    if let Some(n) = row.get("name_value").and_then(|x| x.as_str()) {
                        for part in n.split('\n') {
                            names.insert(part.trim().to_string());
                        }
                    }
                }
                let preview: Vec<_> = names.iter().take(15).cloned().collect();
                findings.push(Finding {
                    engine: "crt.sh".into(),
                    subject: domain.clone(),
                    title: "Certificate transparency".into(),
                    detail: format!("{} names; sample: {}", names.len(), preview.join(", ")),
                    url: Some(format!("https://crt.sh/?q=%25.{domain}")),
                    confidence: Confidence::High,
                    reason: "Public CT logs via crt.sh".into(),
                });
            }
        }
    }

    // WHOIS-like: RDAP if available
    let rdap = format!("https://rdap.org/domain/{}", urlencoding::encode(&domain));
    if let Ok(resp) = client.get(&rdap).send().await {
        if resp.status().is_success() {
            if let Ok(v) = resp.json::<Value>().await {
                let name = v.get("ldhName").and_then(|x| x.as_str()).unwrap_or(&domain);
                let status = v
                    .get("status")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                findings.push(Finding {
                    engine: "rdap".into(),
                    subject: domain.clone(),
                    title: "RDAP domain".into(),
                    detail: format!("{name} status={status}"),
                    url: Some(rdap),
                    confidence: Confidence::High,
                    reason: "Public RDAP".into(),
                });
            }
        }
    }

    if strict {
        findings.retain(|f| f.passes_strict());
    }
    if let Ok(st) = Store::open_default() {
        let _ = st.save_scan("domain", &domain, &findings);
    }
    for f in &findings {
        println!(
            "  [{}] {} — {}",
            f.confidence.as_str(),
            style(&f.title).bold(),
            f.detail
        );
    }
    Ok(findings)
}
