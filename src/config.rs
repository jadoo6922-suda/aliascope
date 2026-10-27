use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Input};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ai_key_primary: String,
    pub ai_key_secondary: String,
    pub hibp_api_key: String,
    pub ai_primary_base: String,
    pub ai_secondary_base: String,
    pub github_token: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai_key_primary: String::new(),
            ai_key_secondary: String::new(),
            hibp_api_key: String::new(),
            ai_primary_base: "https://api.groq.com/openai/v1".into(),
            ai_secondary_base: "https://generativelanguage.googleapis.com/v1beta".into(),
            github_token: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub rate_limit_ms: u64,
    pub max_concurrent: usize,
    pub request_timeout_sec: u64,
    pub user_agent: String,
    pub proxy: Option<String>,
    pub strict_default: bool,
    pub cache_ttl_hours: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rate_limit_ms: 120,
            max_concurrent: 8,
            request_timeout_sec: 12,
            user_agent: "Aliascope/0.3 (+legal-osint)".into(),
            proxy: None,
            strict_default: false,
            cache_ttl_hours: 24,
        }
    }
}

pub fn aliascope_dir() -> Result<PathBuf> {
    let dir = dirs::home_dir().context("home")?.join(".aliascope");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

impl AppConfig {
    pub fn path() -> Result<PathBuf> {
        Ok(aliascope_dir()?.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn save(&self) -> Result<()> {
        fs::write(Self::path()?, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl Settings {
    pub fn load() -> Self {
        let candidates = [
            PathBuf::from("data/settings.yaml"),
            PathBuf::from("settings.yaml"),
            dirs::home_dir()
                .map(|h| h.join(".aliascope/settings.yaml"))
                .unwrap_or_default(),
        ];
        for p in candidates {
            if p.as_os_str().is_empty() || !p.exists() {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(&p) {
                if let Ok(s) = serde_yaml::from_str::<Settings>(&raw) {
                    return s;
                }
            }
        }
        Self::default()
    }
}

pub fn interactive_config() -> Result<()> {
    println!("Local keys → ~/.aliascope/config.json");
    let mut cfg = AppConfig::load()?;
    cfg.ai_key_primary = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Primary AI API key")
        .default(cfg.ai_key_primary.clone())
        .allow_empty(true)
        .interact_text()?;
    cfg.ai_key_secondary = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Secondary AI API key (Gemini)")
        .default(cfg.ai_key_secondary.clone())
        .allow_empty(true)
        .interact_text()?;
    cfg.hibp_api_key = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("HIBP API key (optional)")
        .default(cfg.hibp_api_key.clone())
        .allow_empty(true)
        .interact_text()?;
    cfg.github_token = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("GitHub token optional (public API rate)")
        .default(cfg.github_token.clone())
        .allow_empty(true)
        .interact_text()?;
    cfg.ai_primary_base = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Primary AI base URL")
        .default(cfg.ai_primary_base.clone())
        .interact_text()?;
    cfg.save()?;
    println!("Saved.");
    Ok(())
}
