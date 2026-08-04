use anyhow::{bail, Result};

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

fn authed_client(config: &Config, token: Option<String>) -> Result<ApiClient> {
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };
    Ok(ApiClient::new(&config.server_url, Some(token)))
}

pub async fn list(space: &str, env: &str, show: bool) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    let secrets = client.list_secrets(space, env).await?;

    if secrets.is_empty() {
        println!("No secrets found.");
        return Ok(());
    }

    let key_width = secrets.iter().map(|s| s.key.len()).max().unwrap_or(0).max(3);

    println!("{:<key_width$}  {:<40}  {}", "KEY", "VALUE", "VERSION");

    for s in &secrets {
        let displayed_value = if show {
            s.value.clone()
        } else {
            "*".repeat(s.value.len().min(8))
        };
        println!("{:<key_width$}  {:<40}  {}", s.key, displayed_value, s.version);
    }
    Ok(())
}

pub async fn set(space: &str, env: &str, key: &str, value: &str) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    let secret = client.set_secret(space, env, key, value).await?;
    println!("Set {} (version {})", secret.key, secret.version);
    Ok(())
}

pub async fn get(space: &str, env: &str, key: &str) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    let secret = client.get_secret(space, env, key).await?;
    print!("{}", secret.value);
    Ok(())
}

pub async fn delete(space: &str, env: &str, key: &str) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    client.delete_secret(space, env, key).await?;
    println!("Deleted {}", key);
    Ok(())
}
