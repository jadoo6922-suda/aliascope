use anyhow::{bail, Result};
use console::style;
use md5::{Digest, Md5};
use std::fs;
use std::path::Path;

use crate::confidence::{Confidence, Finding};
use crate::storage::Store;

/// Local image metadata only (file size + hashes). No camera theft / private cloud scraping.
pub async fn scan(path: &str, strict: bool) -> Result<Vec<Finding>> {
    let p = Path::new(path);
    if !p.is_file() {
        bail!("file not found: {path}");
    }
    println!("{}", style(format!("Image engine · {path}")).cyan().bold());
    let bytes = fs::read(p)?;
    let mut findings = Vec::new();

    findings.push(Finding {
        engine: "image".into(),
        subject: path.into(),
        title: "File size".into(),
        detail: format!("{} bytes", bytes.len()),
        url: None,
        confidence: Confidence::High,
        reason: "Local filesystem metadata".into(),
    });

    let mut md5 = Md5::new();
    md5.update(&bytes);
    let md5hex = format!("{:x}", md5.finalize());
    findings.push(Finding {
        engine: "image".into(),
        subject: path.into(),
        title: "MD5".into(),
        detail: md5hex.clone(),
        url: None,
        confidence: Confidence::High,
        reason: "Local hash for correlation (e.g. Gravatar)".into(),
    });

    findings.push(Finding {
        engine: "image".into(),
        subject: path.into(),
        title: "Gravatar reverse hint".into(),
        detail: format!("https://www.gravatar.com/avatar/{md5hex}?d=404"),
        url: Some(format!("https://www.gravatar.com/avatar/{md5hex}?d=404")),
        confidence: Confidence::Info,
        reason: "Only useful if this MD5 is an email hash; not automatic identity".into(),
    });

    // EXIF would need additional crate; keep boundary explicit
    findings.push(Finding {
        engine: "image".into(),
        subject: path.into(),
        title: "EXIF".into(),
        detail: "Not parsed in this build — optional future (local only)".into(),
        url: None,
        confidence: Confidence::Info,
        reason: "Scope limit".into(),
    });

    if strict {
        findings.retain(|f| f.passes_strict());
    }
    if let Ok(st) = Store::open_default() {
        let _ = st.save_scan("image", path, &findings);
    }
    for f in &findings {
        println!("  [{}] {} — {}", f.confidence.as_str(), f.title, f.detail);
    }
    Ok(findings)
}
