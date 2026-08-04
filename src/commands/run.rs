use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::process::{Command, ExitCode};

use crate::api::ApiClient;
use crate::auth;
use crate::cache;
use crate::config::{resolve_space_env, Config};

pub async fn run(
    space: Option<String>,
    env: Option<String>,
    offline: bool,
    command: &[String],
) -> Result<ExitCode> {
    if command.is_empty() {
        bail!("No command provided. Usage: casier run -s <space> -e <env> -- <command...>");
    }

    let (space, env) = resolve_space_env(space, env)?;

    let secrets = if offline {
        cache::load(&space, &env)
            .with_context(|| format!("no cached secrets for {}/{}", space, env))?
            .secrets
    } else {
        fetch_or_cached(&space, &env).await?
    };

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..]);

    for (key, value) in &secrets {
        cmd.env(key, value);
    }

    let status = cmd.status()?;

    Ok(ExitCode::from(
        status.code().unwrap_or(1).min(255).max(0) as u8,
    ))
}

async fn fetch_or_cached(space: &str, env: &str) -> Result<BTreeMap<String, String>> {
    match fetch(space, env).await {
        Ok(secrets) => {
            cache::store(space, env, &secrets);
            Ok(secrets)
        }
        Err(fetch_err) => match cache::load(space, env) {
            Ok(cached) => {
                eprintln!(
                    "casier: server unreachable, using cached secrets from {}",
                    cached.fetched_at
                );
                Ok(cached.secrets)
            }
            Err(_) => Err(fetch_err.context(format!(
                "server unreachable and no cached secrets for {}/{}",
                space, env
            ))),
        },
    }
}

async fn fetch(space: &str, env: &str) -> Result<BTreeMap<String, String>> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let secrets = client.list_secrets(space, env).await?;
    Ok(cache::to_map(&secrets))
}
