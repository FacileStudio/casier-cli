use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::process::{Command, ExitCode};

use crate::api::{ApiClient, MissingValues};
use crate::auth;
use crate::cache;
use crate::config::{resolve_project_env, Config};

pub async fn run(
    project: Option<String>,
    env: Option<String>,
    offline: bool,
    command: &[String],
) -> Result<ExitCode> {
    if command.is_empty() {
        bail!("No command provided. Usage: casier run -s <project> -e <env> -- <command...>");
    }

    let (project, env) = resolve_project_env(project, env)?;

    let secrets = if offline {
        cache::load(&project, &env)
            .with_context(|| format!("no cached secrets for {}/{}", project, env))?
            .secrets
    } else {
        fetch_or_cached(&project, &env).await?
    };

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..]);

    for (key, value) in &secrets {
        cmd.env(key, value);
    }

    let status = cmd.status()?;

    Ok(ExitCode::from(
        status.code().unwrap_or(1).min(255).max(0) as u8
    ))
}

async fn fetch_or_cached(project: &str, env: &str) -> Result<BTreeMap<String, String>> {
    match fetch(project, env).await {
        Ok(secrets) => {
            cache::store(project, env, &secrets);
            Ok(secrets)
        }
        // A server that answered without values was reached perfectly well, and
        // quietly running the command against a stale cache would hide the very
        // problem the guard exists to surface.
        Err(fetch_err) if fetch_err.downcast_ref::<MissingValues>().is_some() => Err(fetch_err),
        Err(fetch_err) => match cache::load(project, env) {
            Ok(cached) => {
                crate::ui::warn(&format!(
                    "server unreachable, using cached secrets from {}",
                    cached.fetched_at
                ));
                Ok(cached.secrets)
            }
            Err(_) => Err(fetch_err.context(format!(
                "server unreachable and no cached secrets for {}/{}",
                project, env
            ))),
        },
    }
}

async fn fetch(project: &str, env: &str) -> Result<BTreeMap<String, String>> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("not logged in — run `casier login`");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let secrets = client.reveal_secrets(project, env).await?;
    Ok(cache::to_map(&secrets))
}
