use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Platform {
    pub name: String,
    pub url: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct PlatformFile {
    platforms: Vec<Platform>,
}

pub fn load_platforms() -> Result<Vec<Platform>> {
    let candidates = [
        PathBuf::from("data/platforms.yaml"),
        PathBuf::from("platforms.yaml"),
        dirs::home_dir()
            .map(|h| h.join(".aliascope/platforms.yaml"))
            .unwrap_or_default(),
    ];
    for path in candidates {
        if path.as_os_str().is_empty() || !path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let file: PlatformFile =
            serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        let list: Vec<_> = file.platforms.into_iter().filter(|p| p.enabled).collect();
        if !list.is_empty() {
            return Ok(list);
        }
    }
    Ok(vec![Platform {
        name: "GitHub".into(),
        url: "https://github.com/{user}".into(),
        enabled: true,
    }])
}

pub fn profile_url(template: &str, user: &str) -> String {
    template.replace("{user}", user)
}
