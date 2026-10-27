use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::confidence::Finding;

#[derive(Serialize)]
struct Report {
    tool: &'static str,
    version: &'static str,
    kind: String,
    query: String,
    generated_at: String,
    finding_count: usize,
    findings: Vec<Finding>,
    disclaimer: &'static str,
}

pub fn write_json(path: &Path, kind: &str, query: &str, findings: &[Finding]) -> Result<()> {
    let report = Report {
        tool: "aliascope",
        version: env!("CARGO_PKG_VERSION"),
        kind: kind.into(),
        query: query.into(),
        generated_at: Utc::now().to_rfc3339(),
        finding_count: findings.len(),
        findings: findings.to_vec(),
        disclaimer: "Public signals only. Not identity proof. No passwords.",
    };
    fs::write(path, serde_json::to_string_pretty(&report)?)?;
    Ok(())
}

pub fn write_csv(path: &Path, findings: &[Finding]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
        "engine",
        "subject",
        "title",
        "detail",
        "url",
        "confidence",
        "reason",
    ])?;
    for f in findings {
        wtr.write_record([
            f.engine.as_str(),
            f.subject.as_str(),
            f.title.as_str(),
            f.detail.as_str(),
            f.url.as_deref().unwrap_or(""),
            f.confidence.as_str(),
            f.reason.as_str(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn write_html(path: &Path, kind: &str, query: &str, findings: &[Finding]) -> Result<()> {
    let mut rows = String::new();
    for f in findings {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            esc(&f.engine),
            esc(&f.subject),
            esc(&f.title),
            esc(&f.detail),
            f.url
                .as_ref()
                .map(|u| format!("<a href=\"{}\">link</a>", esc(u)))
                .unwrap_or_else(|| "-".into()),
            f.confidence.as_str(),
            esc(&f.reason),
        ));
    }
    if rows.is_empty() {
        rows = "<tr><td colspan=\"7\">No findings</td></tr>".into();
    }
    let html = format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/>
<title>Aliascope {kind} — {q}</title>
<style>
body{{font-family:system-ui,sans-serif;background:#0b1220;color:#e8eefc;margin:2rem}}
h1{{color:#6ecbff}} table{{border-collapse:collapse;width:100%}}
th,td{{border:1px solid #243044;padding:.5rem;text-align:left;font-size:.9rem}}
th{{background:#152238}} a{{color:#7dd3fc}} .warn{{color:#fbbf24;margin-top:1rem}}
</style></head><body>
<h1>Aliascope {kind}</h1>
<p>Query: <strong>{q}</strong> · {n} findings · {ts}</p>
<table><thead><tr>
<th>Engine</th><th>Subject</th><th>Title</th><th>Detail</th><th>URL</th><th>Confidence</th><th>Reason</th>
</tr></thead><tbody>{rows}</tbody></table>
<p class="warn">Public signals only. Not identity proof. No passwords. Authorized use only.</p>
</body></html>"#,
        kind = esc(kind),
        q = esc(query),
        n = findings.len(),
        ts = esc(&Utc::now().to_rfc3339()),
        rows = rows
    );
    fs::write(path, html)?;
    Ok(())
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn export_all(
    kind: &str,
    query: &str,
    findings: &[Finding],
    json: Option<&Path>,
    csv: Option<&Path>,
    html: Option<&Path>,
) -> Result<()> {
    if let Some(p) = json {
        write_json(p, kind, query, findings)?;
    }
    if let Some(p) = csv {
        write_csv(p, findings)?;
    }
    if let Some(p) = html {
        write_html(p, kind, query, findings)?;
    }
    Ok(())
}
