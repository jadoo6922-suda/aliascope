use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Primary AI key (e.g. Groq / OpenAI compatible)
    pub ai_key_primary: String,
    /// Secondary AI key (e.g. Gemini)
    pub ai_key_secondary: String,
    /// Optional Have I Been Pwned API key for email breach checks
    pub hibp_api_key: String,
    pub ai_primary_base: String,
    pub ai_secondary_base: String,
}

impl AppConfig {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::home_dir()
            .context("cannot resolve home directory")?
            .join(".aliascope");
        fs::create_dir_all(&dir)?;
        Ok(dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self {
                ai_primary_base: "https://api.groq.com/openai/v1".into(),
                ai_secondary_base: "https://generativelanguage.googleapis.com/v1beta".into(),
                ..Default::default()
            });
        }
        let raw = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}

pub fn interactive_config() -> Result<()> {
    println!("{}", style("Local configuration (~/.aliascope/config.json)").green().bold());
    println!("{}", style("Keys stay on this machine. Never commit them to git.").dim());

    let mut cfg = AppConfig::load()?;

    cfg.ai_key_primary = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Primary AI API key (Groq/OpenAI-compatible)")
        .default(cfg.ai_key_primary.clone())
        .allow_empty(true)
        .interact_text()?;

    cfg.ai_key_secondary = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Secondary AI API key (Gemini)")
        .default(cfg.ai_key_secondary.clone())
        .allow_empty(true)
        .interact_text()?;

    cfg.hibp_api_key = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Have I Been Pwned API key (optional but recommended)")
        .default(cfg.hibp_api_key.clone())
        .allow_empty(true)
        .interact_text()?;

    cfg.ai_primary_base = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Primary AI base URL")
        .default(cfg.ai_primary_base.clone())
        .interact_text()?;

    cfg.ai_secondary_base = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Secondary AI base URL")
        .default(cfg.ai_secondary_base.clone())
        .interact_text()?;

    cfg.save()?;
    println!("{}", style("Saved.").green());
    Ok(())
}
