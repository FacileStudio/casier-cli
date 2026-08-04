use anyhow::{bail, Result};
use std::process::{Command, ExitCode};

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

pub async fn run(space: &str, env: &str, command: &[String]) -> Result<ExitCode> {
    if command.is_empty() {
        bail!("No command provided. Usage: casier run -s <space> -e <env> -- <command...>");
    }

    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let secrets = client.list_secrets(space, env).await?;

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..]);

    for secret in &secrets {
        cmd.env(&secret.key, &secret.value);
    }

    let status = cmd.status()?;

    Ok(ExitCode::from(
        status.code().unwrap_or(1).min(255).max(0) as u8,
    ))
}
