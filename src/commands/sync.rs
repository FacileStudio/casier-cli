use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::api::ApiClient;
use crate::auth;
use crate::cache;
use crate::config::Config;
use crate::envfile;

pub async fn push(project: &str, env: &str, file: &str) -> Result<()> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("not logged in — run `casier login`");
    };

    let path = PathBuf::from(file);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let client = ApiClient::new(&config.server_url, Some(token));
    let resp = client.import_env(project, env, &content).await?;

    crate::ui::success(&format!(
        "Imported {} created, {} updated, {} skipped",
        resp.created, resp.updated, resp.skipped
    ));
    Ok(())
}

pub async fn pull(project: &str, env: &str, file: &str) -> Result<()> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("not logged in — run `casier login`");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let resp = client.export_env(project, env).await?;

    // Writing an empty file is destructive and indistinguishable from success,
    // so an empty export stops here rather than truncating whatever the target
    // already held.
    if resp.content.trim().is_empty() {
        bail!(
            "{}/{} exported nothing — refusing to overwrite {} with an empty file",
            project,
            env,
            file
        );
    }

    cache::store(project, env, &envfile::parse(&resp.content));

    let path = PathBuf::from(file);
    std::fs::write(&path, &resp.content)
        .with_context(|| format!("failed to write {}", path.display()))?;

    let line_count = resp.content.lines().count();
    crate::ui::success(&format!(
        "Pulled {} secrets to {}",
        line_count,
        path.display()
    ));
    Ok(())
}
