use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::process::ExitCode;

use crate::api::ApiClient;
use crate::auth;
use crate::cache;
use crate::config::{resolve_space_env, Config};
use crate::envfile;

pub async fn run(file: &str, space: Option<String>, env: Option<String>) -> Result<ExitCode> {
    let (space, env) = resolve_space_env(space, env)?;

    let content =
        std::fs::read_to_string(file).with_context(|| format!("failed to read {}", file))?;
    let local: BTreeSet<String> = envfile::parse(&content).into_keys().collect();

    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let secrets = client.list_secrets(&space, &env).await?;
    cache::store(&space, &env, &cache::to_map(&secrets));

    let remote: BTreeSet<String> = secrets.into_iter().map(|s| s.key).collect();
    let (missing_remote, missing_local) = compare(&local, &remote);

    if missing_remote.is_empty() && missing_local.is_empty() {
        println!("{} is in sync with {}/{}.", file, space, env);
        return Ok(ExitCode::SUCCESS);
    }

    for key in &missing_remote {
        println!("- {}  (in {} but missing from {}/{})", key, file, space, env);
    }
    for key in &missing_local {
        println!("+ {}  (in {}/{} but missing from {})", key, space, env, file);
    }

    if missing_remote.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn compare(local: &BTreeSet<String>, remote: &BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let missing_remote = local.difference(remote).cloned().collect();
    let missing_local = remote.difference(local).cloned().collect();
    (missing_remote, missing_local)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn compare_reports_both_directions() {
        let local = set(&["A", "B", "C"]);
        let remote = set(&["B", "C", "D"]);
        let (missing_remote, missing_local) = compare(&local, &remote);
        assert_eq!(missing_remote, vec!["A".to_string()]);
        assert_eq!(missing_local, vec!["D".to_string()]);
    }

    #[test]
    fn compare_in_sync() {
        let keys = set(&["X", "Y"]);
        let (missing_remote, missing_local) = compare(&keys, &keys);
        assert!(missing_remote.is_empty());
        assert!(missing_local.is_empty());
    }
}
