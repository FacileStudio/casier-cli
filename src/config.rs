use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_server_url")]
    pub server_url: String,
}

pub fn default_server_url() -> String {
    "http://localhost:4000".to_string()
}

pub fn normalize_server_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("server URL cannot be empty");
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(trimmed.to_string());
    }
    Ok(format!("https://{}", trimmed))
}

impl Config {
    pub fn dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("cannot determine config directory")?;
        Ok(config_dir.join("casier"))
    }

    pub fn path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        if let Ok(url) = std::env::var("CASIER_SERVER_URL") {
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

    pub fn stored() -> Option<Self> {
        let contents = std::fs::read_to_string(Self::path().ok()?).ok()?;
        toml::from_str(&contents).ok()
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::dir()?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create {}", dir.display()))?;
        let path = Self::path()?;
        let body = toml::to_string(self).context("cannot serialize config")?;
        std::fs::write(&path, body).with_context(|| format!("cannot write {}", path.display()))?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
}

#[derive(Debug, Deserialize)]
pub struct ProjectSection {
    pub slug: String,
    #[serde(default = "default_env")]
    pub environment: String,
}

fn default_env() -> String {
    "dev".to_string()
}

impl ProjectConfig {
    pub fn load() -> Option<Self> {
        let path = std::path::PathBuf::from(".casier.toml");
        if !path.exists() {
            return None;
        }
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }
}

pub fn resolve_project_env(
    project: Option<String>,
    env: Option<String>,
) -> Result<(String, String)> {
    let local = ProjectConfig::load();
    let project = project
        .or_else(|| local.as_ref().map(|c| c.project.slug.clone()))
        .context("no project specified: pass --project or run `casier init`")?;
    let env = env
        .or_else(|| local.as_ref().map(|c| c.project.environment.clone()))
        .unwrap_or_else(default_env);
    Ok((project, env))
}
