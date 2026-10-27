use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::username::Hit;

#[derive(Serialize)]
pub struct UserScanReport {
    pub tool: &'static str,
    pub version: &'static str,
    pub kind: &'static str,
    pub query: String,
    pub generated_at: String,
    pub hit_count: usize,
    pub hits: Vec<HitExport>,
    pub disclaimer: &'static str,
}

#[derive(Serialize)]
pub struct HitExport {
    pub alias: String,
    pub platform: String,
    pub url: String,
    pub status: u16,
    pub confidence: String,
}

pub fn hits_to_export(hits: &[Hit]) -> Vec<HitExport> {
    hits.iter()
        .map(|h| HitExport {
            alias: h.alias.clone(),
            platform: h.platform.clone(),
            url: h.url.clone(),
            status: h.status,
            confidence: if h.likely_exists {
                "medium".into()
            } else {
                "low".into()
            },
        })
        .collect()
}

pub fn write_json(path: &Path, query: &str, hits: &[Hit]) -> Result<()> {
    let report = UserScanReport {
        tool: "aliascope",
        version: env!("CARGO_PKG_VERSION"),
        kind: "user",
        query: query.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        hit_count: hits.len(),
        hits: hits_to_export(hits),
        disclaimer: "Public URL signals only. Not identity proof. No passwords.",
    };
    let raw = serde_json::to_string_pretty(&report)?;
    fs::write(path, raw)?;
    Ok(())
}

pub fn write_html(path: &Path, query: &str, hits: &[Hit]) -> Result<()> {
    let mut rows = String::new();
    for h in hits {
        let conf = if h.likely_exists { "medium" } else { "low" };
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td></tr>\n",
            escape(&h.platform),
            escape(&h.alias),
            escape(&h.url),
            escape(&h.url),
            h.status,
            conf
        ));
    }
    if rows.is_empty() {
        rows = "<tr><td colspan=\"5\">No hits</td></tr>".into();
    }
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>Aliascope report — {q}</title>
<style>
  body {{ font-family: system-ui, sans-serif; background:#0b1220; color:#e8eefc; margin:2rem; }}
  h1 {{ color:#6ecbff; }}
  .meta {{ color:#9fb0c7; margin-bottom:1.5rem; }}
  table {{ border-collapse: collapse; width:100%; }}
  th, td {{ border:1px solid #243044; padding:.55rem .7rem; text-align:left; }}
  th {{ background:#152238; }}
  tr:nth-child(even) {{ background:#101a2b; }}
  a {{ color:#7dd3fc; }}
  .warn {{ margin-top:1.5rem; color:#fbbf24; font-size:.9rem; }}
</style>
</head>
<body>
  <h1>Aliascope scan</h1>
  <div class="meta">
    Query: <strong>{q}</strong><br/>
    Generated: {ts}<br/>
    Hits: {n}
  </div>
  <table>
    <thead><tr><th>Platform</th><th>Alias</th><th>URL</th><th>HTTP</th><th>Confidence</th></tr></thead>
    <tbody>
{rows}
    </tbody>
  </table>
  <p class="warn">Public URL signals only. Not identity proof. No passwords. Use only with authorization.</p>
</body>
</html>
"#,
        q = escape(query),
        ts = escape(&Utc::now().to_rfc3339()),
        n = hits.len(),
        rows = rows
    );
    fs::write(path, html)?;
    Ok(())
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
