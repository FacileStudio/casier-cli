use anyhow::{bail, Result};

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

fn authed_client(config: &Config, token: Option<String>) -> Result<ApiClient> {
    let Some(token) = token else {
        bail!("not logged in — run `casier login`");
    };
    Ok(ApiClient::new(&config.server_url, Some(token)))
}

/// A masked row shows a fixed number of stars, not one per character: the
/// length of a secret is itself something worth not leaking, and without
/// `--show` the CLI no longer asks the server for the value at all.
const MASK: &str = "********";

pub async fn list(project: &str, env: &str, show: bool) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;

    let rows: Vec<(String, String, i32)> = if show {
        client
            .reveal_secrets(project, env)
            .await?
            .into_iter()
            .map(|s| (s.key, s.value, s.version))
            .collect()
    } else {
        client
            .list_secrets(project, env)
            .await?
            .into_iter()
            .map(|s| (s.key, MASK.to_string(), s.version))
            .collect()
    };

    if rows.is_empty() {
        crate::ui::step("No secrets");
        return Ok(());
    }

    let key_width = rows
        .iter()
        .map(|(key, _, _)| key.len())
        .max()
        .unwrap_or(0)
        .max(3);

    println!("{:<key_width$}  {:<40}  {}", "KEY", "VALUE", "VERSION");

    for (key, value, version) in &rows {
        println!("{:<key_width$}  {:<40}  {}", key, value, version);
    }
    Ok(())
}

pub async fn set(project: &str, env: &str, key: &str, value: &str) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    let secret = client.set_secret(project, env, key, value).await?;
    crate::ui::success(&format!("Set {} (version {})", secret.key, secret.version));
    Ok(())
}

pub async fn get(project: &str, env: &str, key: &str) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    let secret = client.reveal_secret(project, env, key).await?;
    print!("{}", secret.value);
    Ok(())
}

pub async fn delete(project: &str, env: &str, key: &str) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    client.delete_secret(project, env, key).await?;
    crate::ui::success(&format!("Deleted {}", key));
    Ok(())
}
