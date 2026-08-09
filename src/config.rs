use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_server_url")]
    pub server_url: String,
}

pub fn default_server_url() -> String {
    "https://casier.facile.studio/api".to_string()
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

/// The per-repository config filenames, in priority order. The first is what
/// `casier init` writes.
///
/// `.casier.toml` is still read so a repository that has not been converted keeps
/// working — renaming these across a suite cannot be atomic, and a developer on
/// an older checkout must not lose their dev loop over it.
pub const PROJECT_CONFIG_NAMES: [&str; 3] = ["casier.yml", "casier.yaml", ".casier.toml"];

impl ProjectConfig {
    /// Reads the project config from the working directory.
    ///
    /// `Ok(None)` means no file at all. A file that exists but does not parse is
    /// an error rather than a `None`: falling through would surface as
    /// "no project specified", which sends the reader to look for a missing file
    /// instead of at the typo in the one they have.
    pub fn load() -> Result<Option<Self>> {
        Self::load_from(std::path::Path::new("."))
    }

    pub fn load_from(dir: &std::path::Path) -> Result<Option<Self>> {
        for name in PROJECT_CONFIG_NAMES {
            let path = dir.join(name);
            if !path.exists() {
                continue;
            }
            let contents =
                std::fs::read_to_string(&path).with_context(|| format!("cannot read {}", name))?;
            let parsed = if name.ends_with(".toml") {
                toml::from_str(&contents).map_err(anyhow::Error::from)
            } else {
                serde_norway::from_str(&contents).map_err(anyhow::Error::from)
            };
            return parsed
                .map(Some)
                .with_context(|| format!("invalid {}", name));
        }
        Ok(None)
    }
}

pub fn resolve_project_env(
    project: Option<String>,
    env: Option<String>,
) -> Result<(String, String)> {
    let local = ProjectConfig::load()?;
    let project = project
        .or_else(|| local.as_ref().map(|c| c.project.slug.clone()))
        .context("no project specified: pass --project or run `casier init`")?;
    let env = env
        .or_else(|| local.as_ref().map(|c| c.project.environment.clone()))
        .unwrap_or_else(default_env);
    Ok((project, env))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scratch directory is keyed on the calling test, not on the filename:
    /// cargo runs tests in parallel, and two cases writing the same name into a
    /// shared directory read each other's file.
    fn scratch(case: &str, name: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("casier-cfg-{}", case));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), body).unwrap();
        dir
    }

    #[test]
    fn reads_yaml() {
        let dir = scratch(
            "reads_yaml",
            "casier.yml",
            "project:\n  slug: antenne\n  environment: dev\n",
        );
        let cfg = ProjectConfig::load_from(&dir).unwrap().expect("a config");
        assert_eq!(cfg.project.slug, "antenne");
        assert_eq!(cfg.project.environment, "dev");
    }

    #[test]
    fn still_reads_the_legacy_toml() {
        let dir = scratch(
            "legacy_toml",
            ".casier.toml",
            "[project]\nslug = \"sablier\"\n",
        );
        let cfg = ProjectConfig::load_from(&dir).unwrap().expect("a config");
        assert_eq!(cfg.project.slug, "sablier");
        assert_eq!(cfg.project.environment, "dev", "environment still defaults");
    }

    #[test]
    fn yaml_wins_over_the_legacy_toml() {
        let dir = scratch("yaml_wins", "casier.yml", "project:\n  slug: new\n");
        std::fs::write(dir.join(".casier.toml"), "[project]\nslug = \"old\"\n").unwrap();
        let cfg = ProjectConfig::load_from(&dir).unwrap().expect("a config");
        assert_eq!(cfg.project.slug, "new");
    }

    #[test]
    fn no_file_is_not_an_error() {
        let dir = scratch("no_file", "unrelated.txt", "");
        assert!(ProjectConfig::load_from(&dir).unwrap().is_none());
    }

    #[test]
    fn a_broken_file_is_loud() {
        let dir = scratch(
            "broken",
            "casier.yml",
            "project:\n  slug: [this is not a string\n",
        );
        let err = ProjectConfig::load_from(&dir).unwrap_err();
        assert!(
            err.to_string().contains("casier.yml"),
            "the error must name the file, got: {err}"
        );
    }
}
