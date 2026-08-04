use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::api::ApiClient;
use crate::auth;
use crate::cache;
use crate::config::Config;
use crate::envfile;

pub async fn push(space: &str, env: &str, file: &str) -> Result<()> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let path = PathBuf::from(file);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let client = ApiClient::new(&config.server_url, Some(token));
    let resp = client.import_env(space, env, &content).await?;

    println!(
        "Imported: {} created, {} updated, {} skipped",
        resp.created, resp.updated, resp.skipped
    );
    Ok(())
}

pub async fn pull(space: &str, env: &str, file: &str) -> Result<()> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let resp = client.export_env(space, env).await?;

    cache::store(space, env, &envfile::parse(&resp.content));

    let path = PathBuf::from(file);
    std::fs::write(&path, &resp.content)
        .with_context(|| format!("failed to write {}", path.display()))?;

    let line_count = resp.content.lines().count();
    println!("Pulled {} secrets to {}", line_count, path.display());
    Ok(())
}
