use anyhow::{bail, Result};
use console::style;
use md5::{Digest, Md5};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::config::AppConfig;

#[derive(Debug)]
struct FreeHit {
    service: String,
    detail: String,
    confidence: &'static str,
}

fn client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("Aliascope/0.1 (+legal-osint; free-registration-check)")
        .build()?)
}

fn gravatar_hash(email: &str) -> String {
    let norm = email.trim().to_lowercase();
    let mut hasher = Md5::new();
    hasher.update(norm.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Free techniques: public registration / avatar signals (no passwords, no paid API).
async fn free_registration_scan(email: &str) -> Result<Vec<FreeHit>> {
    let c = client()?;
    let mut hits = Vec::new();
    let encoded = urlencoding::encode(email);

    // 1) Gravatar — free, official public hash URL
    let gh = gravatar_hash(email);
    let gravatar_url = format!("https://www.gravatar.com/avatar/{gh}?d=404");
    match c.get(&gravatar_url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => {
            hits.push(FreeHit {
                service: "Gravatar".into(),
                detail: format!("Public avatar exists for hash {gh}"),
                confidence: "high",
            });
        }
        Ok(_) => {}
        Err(_) => {}
    }
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 2) GitHub password-reset style endpoint often returns different body lengths
    //    We only record "possible" signals — never claim definitive account ownership.
    let gh_url = "https://github.com/sessions/password_reset";
    if let Ok(resp) = c
        .post(gh_url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(format!("email={encoded}"))
        .send()
        .await
    {
        let status = resp.status().as_u16();
        // 200 page returned — presence is ambiguous; note only as probe completed
        if status == 200 || status == 422 {
            hits.push(FreeHit {
                service: "GitHub".into(),
                detail: format!(
                    "Password-reset probe HTTP {status} (ambiguous; verify manually)"
                ),
                confidence: "low",
            });
        }
    }
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 3) Adobe — classic OSINT registration probe (public form response differs)
    let adobe = format!(
        "https://auth.services.adobe.com/signin/v2/users/authorization?email={encoded}"
    );
    if let Ok(resp) = c.get(&adobe).send().await {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        let lower = body.to_lowercase();
        if status == 200
            && (lower.contains("registered")
                || lower.contains("\"error\"")
                || lower.contains("account"))
        {
            hits.push(FreeHit {
                service: "Adobe".into(),
                detail: format!("Registration-related response HTTP {status}"),
                confidence: "medium",
            });
        }
    }
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 4) Tumblr / blogspot style public check via mailru-like is unstable;
    //    Use DuckDuckGo !email is not reliable. Instead: Spotify-ish public
    //    status endpoints change often — keep a conservative additional check:

    // Instagram does not expose simple free email check without login — skip.

    // 5) Mozilla / FXA — may rate-limit; soft probe
    let fxa = format!(
        "https://api.accounts.firefox.com/v1/account/status?email={encoded}"
    );
    if let Ok(resp) = c.get(&fxa).send().await {
        let status = resp.status().as_u16();
        if let Ok(v) = resp.json::<Value>().await {
            if let Some(exists) = v.get("exists").and_then(|x| x.as_bool()) {
                if exists {
                    hits.push(FreeHit {
                        service: "Firefox Accounts".into(),
                        detail: "API reports exists=true".into(),
                        confidence: "high",
                    });
                } else {
                    hits.push(FreeHit {
                        service: "Firefox Accounts".into(),
                        detail: "API reports exists=false".into(),
                        confidence: "high",
                    });
                }
            } else if status == 200 {
                hits.push(FreeHit {
                    service: "Firefox Accounts".into(),
                    detail: format!("HTTP {status} JSON without exists field"),
                    confidence: "low",
                });
            }
        }
    }
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 6) OpenStreetMap / board style — skip fragile ones.

    // 7) Always offer free manual HIBP link
    hits.push(FreeHit {
        service: "Have I Been Pwned (manual)".into(),
        detail: format!("https://haveibeenpwned.com/account/{email}"),
        confidence: "info",
    });

    Ok(hits)
}

async fn hibp_paid_optional(email: &str, api_key: &str) -> Result<()> {
    let c = client()?;
    let url = format!(
        "https://haveibeenpwned.com/api/v3/breachedaccount/{}?truncateResponse=false",
        urlencoding::encode(email)
    );
    let resp = c
        .get(&url)
        .header("hibp-api-key", api_key)
        .header("user-agent", "Aliascope/0.1")
        .send()
        .await?;
    let status = resp.status().as_u16();
    match status {
        200 => {
            let body: Value = resp.json().await?;
            println!(
                "{}",
                style("HIBP: address APPEARS in known breaches.")
                    .red()
                    .bold()
            );
            if let Some(arr) = body.as_array() {
                for b in arr.iter().take(20) {
                    let name = b.get("Name").and_then(|v| v.as_str()).unwrap_or("?");
                    let domain = b.get("Domain").and_then(|v| v.as_str()).unwrap_or("-");
                    let date = b.get("BreachDate").and_then(|v| v.as_str()).unwrap_or("-");
                    println!("  • {name} ({domain}) date={date}");
                }
            }
            println!(
                "{}",
                style("No passwords are returned by this API path.").dim()
            );
        }
        404 => {
            println!(
                "{}",
                style("HIBP: not found in their breach dataset.")
                    .green()
                    .bold()
            );
        }
        401 => bail!("HIBP API key rejected."),
        429 => bail!("HIBP rate limit — wait and retry."),
        other => bail!("HIBP HTTP {other}"),
    }
    Ok(())
}

/// Main entry: always run free scan; optionally enrich with HIBP if key configured.
pub async fn run_email_check(email: &str) -> Result<()> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 5 {
        bail!("Please provide a valid email address.");
    }

    println!(
        "{}",
        style(format!("Email OSINT for {email}")).cyan().bold()
    );
    println!(
        "{}",
        style("Free layer: public registration / avatar signals (no passwords).")
            .dim()
    );
    println!(
        "{}",
        style("Optional layer: HIBP API if you configured a key (breach names only).")
            .dim()
    );
    println!();

    println!("{}", style("▶ Free registration / presence scan").green().bold());
    let hits = free_registration_scan(&email).await?;
    if hits.is_empty() {
        println!("  No free-layer signals.");
    } else {
        for h in &hits {
            println!(
                "  {} {} — {} [{}]",
                style("•").cyan(),
                style(&h.service).bold(),
                h.detail,
                h.confidence
            );
        }
    }

    println!();
    let cfg = AppConfig::load()?;
    if !cfg.hibp_api_key.trim().is_empty() {
        println!("{}", style("▶ HIBP (optional paid key)").magenta().bold());
        hibp_paid_optional(&email, cfg.hibp_api_key.trim()).await?;
    } else {
        println!(
            "{}",
            style("HIBP API key not set — skipped (no subscription required for free layer).")
                .yellow()
        );
        println!(
            "  Manual free check: https://haveibeenpwned.com/account/{}",
            urlencoding::encode(&email)
        );
    }

    println!();
    println!(
        "{}",
        style("Reminder: registration signals ≠ identity proof; never used for account takeover.")
            .dim()
    );
    Ok(())
}
