use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_server_url")]
    pub server_url: String,
}

fn default_server_url() -> String {
    "http://localhost:4000".to_string()
}

impl Config {
    pub fn dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("cannot determine config directory")?;
        Ok(config_dir.join("clef"))
    }

    pub fn path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        if let Ok(url) = std::env::var("CLEF_SERVER_URL") {
            return Ok(Config { server_url: url });
        }

        let path = Self::path()?;
        if !path.exists() {
            return Ok(Config {
                server_url: default_server_url(),
            });
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("invalid config at {}", path.display()))?;
        Ok(config)
    }

}
