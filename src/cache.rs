use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::api::RevealedSecret;
use crate::config::Config;

#[derive(Serialize, Deserialize)]
pub struct CachedSecrets {
    pub fetched_at: String,
    pub secrets: BTreeMap<String, String>,
}

/// Only a revealed list can be cached. `casier run --offline` reads this back
/// verbatim, so a cache built from a valueless response would keep injecting an
/// empty environment long after the response that caused it was forgotten.
pub fn to_map(secrets: &[RevealedSecret]) -> BTreeMap<String, String> {
    secrets
        .iter()
        .map(|s| (s.key.clone(), s.value.clone()))
        .collect()
}

fn cache_dir() -> Result<PathBuf> {
    Ok(Config::dir()?.join("cache"))
}

fn cache_file(dir: &Path, project: &str, env: &str) -> PathBuf {
    dir.join(format!("{}-{}.json", project, env))
}

pub fn store(project: &str, env: &str, secrets: &BTreeMap<String, String>) {
    let result = cache_dir().and_then(|dir| write_to(&dir, project, env, secrets));
    if let Err(e) = result {
        eprintln!("casier: failed to write cache: {:#}", e);
    }
}

pub fn load(project: &str, env: &str) -> Result<CachedSecrets> {
    read_from(&cache_dir()?, project, env)
}

fn write_to(
    dir: &Path,
    project: &str,
    env: &str,
    secrets: &BTreeMap<String, String>,
) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    set_mode(dir, 0o700)?;

    let cached = CachedSecrets {
        fetched_at: now_utc(),
        secrets: secrets.clone(),
    };
    let path = cache_file(dir, project, env);
    let json = serde_json::to_string_pretty(&cached)?;
    std::fs::write(&path, json).with_context(|| format!("cannot write {}", path.display()))?;
    set_mode(&path, 0o600)?;
    Ok(())
}

fn read_from(dir: &Path, project: &str, env: &str) -> Result<CachedSecrets> {
    let path = cache_file(dir, project, env);
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("no cached secrets at {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("invalid cache at {}", path.display()))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .with_context(|| format!("cannot set permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn now_utc() -> String {
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(epoch)
}

fn format_utc(epoch: u64) -> String {
    let (year, month, day) = civil_from_days((epoch / 86400) as i64);
    let secs = epoch % 86400;
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year,
        month,
        day,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_round_trip() {
        let dir = std::env::temp_dir().join(format!("casier-cache-test-{}", std::process::id()));
        let mut secrets = BTreeMap::new();
        secrets.insert("API_KEY".to_string(), "abc123".to_string());
        secrets.insert("DB_URL".to_string(), "postgres://x".to_string());

        write_to(&dir, "myproject", "dev", &secrets).unwrap();
        let cached = read_from(&dir, "myproject", "dev").unwrap();

        assert_eq!(cached.secrets, secrets);
        assert!(!cached.fetched_at.is_empty());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(cache_file(&dir, "myproject", "dev"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cache_missing_is_error() {
        let dir = std::env::temp_dir().join(format!("casier-cache-missing-{}", std::process::id()));
        assert!(read_from(&dir, "nope", "dev").is_err());
    }

    #[test]
    fn formats_epoch_as_utc() {
        assert_eq!(format_utc(1_700_000_000), "2023-11-14 22:13:20 UTC");
        assert_eq!(format_utc(0), "1970-01-01 00:00:00 UTC");
    }
}
