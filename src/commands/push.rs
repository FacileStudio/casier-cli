use anyhow::{bail, Context, Result};

use crate::api::ApiClient;
use crate::auth;
use crate::cache;
use crate::config::{resolve_project_env, Config};

pub async fn dokploy(compose_id: &str, project: Option<String>, env: Option<String>) -> Result<()> {
    let (project, env) = resolve_project_env(project, env)?;

    let dokploy_url = std::env::var("DOKPLOY_URL")
        .context("DOKPLOY_URL is not set (e.g. https://gare.facile.studio)")?;
    let api_key = std::env::var("DOKPLOY_API_KEY").context("DOKPLOY_API_KEY is not set")?;

    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("not logged in — run `casier login`");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let secrets = client.reveal_secrets(&project, &env).await?;
    if secrets.is_empty() {
        bail!(
            "{}/{} has no secrets to push — refusing to overwrite the Dokploy environment of compose {} with an empty block",
            project,
            env,
            compose_id
        );
    }
    cache::store(&project, &env, &cache::to_map(&secrets));

    let env_content = secrets
        .iter()
        .map(|s| format!("{}={}", s.key, s.value))
        .collect::<Vec<_>>()
        .join("\n");

    let url = format!(
        "{}/api/compose.saveEnvironment",
        dokploy_url.trim_end_matches('/')
    );
    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-api-key", &api_key)
        .json(&serde_json::json!({ "composeId": compose_id, "env": env_content }))
        .send()
        .await
        .context("failed to reach Dokploy")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("POST {} failed ({}): {}", url, status, body);
    }

    crate::ui::success(&format!(
        "Pushed {} vars to Dokploy compose {}",
        secrets.len(),
        compose_id
    ));
    Ok(())
}
