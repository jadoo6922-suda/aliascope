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

fn builtin_platforms() -> Vec<Platform> {
    vec![
        Platform {
            name: "GitHub".into(),
            url: "https://github.com/{user}".into(),
            enabled: true,
        },
        Platform {
            name: "GitLab".into(),
            url: "https://gitlab.com/{user}".into(),
            enabled: true,
        },
        Platform {
            name: "Reddit".into(),
            url: "https://www.reddit.com/user/{user}".into(),
            enabled: true,
        },
        Platform {
            name: "X (Twitter)".into(),
            url: "https://x.com/{user}".into(),
            enabled: true,
        },
        Platform {
            name: "Instagram".into(),
            url: "https://www.instagram.com/{user}/".into(),
            enabled: true,
        },
        Platform {
            name: "TikTok".into(),
            url: "https://www.tiktok.com/@{user}".into(),
            enabled: true,
        },
        Platform {
            name: "YouTube".into(),
            url: "https://www.youtube.com/@{user}".into(),
            enabled: true,
        },
        Platform {
            name: "LinkedIn".into(),
            url: "https://www.linkedin.com/in/{user}".into(),
            enabled: true,
        },
        Platform {
            name: "Keybase".into(),
            url: "https://keybase.io/{user}".into(),
            enabled: true,
        },
    ]
}

/// Search order: ./data/platforms.yaml → ~/.aliascope/platforms.yaml → built-in list
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
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let file: PlatformFile = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing {}", path.display()))?;
        let list: Vec<Platform> = file.platforms.into_iter().filter(|p| p.enabled).collect();
        if !list.is_empty() {
            return Ok(list);
        }
    }
    Ok(builtin_platforms())
}

pub fn profile_url(template: &str, user: &str) -> String {
    template.replace("{user}", user)
}
