use anyhow::{bail, Result};
use std::io::{self, IsTerminal, Write};

use crate::api::{ApiClient, CreateKeyRequest};
use crate::auth;
use crate::config::Config;

fn authed_client(config: &Config, token: Option<String>) -> Result<ApiClient> {
    let Some(token) = token else {
        bail!("not logged in, run `casier login`");
    };
    Ok(ApiClient::new(&config.server_url, Some(token)))
}

pub async fn list(app: Option<&str>, json: bool) -> Result<()> {
    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    let keys = client.list_keys(app).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&keys)?);
        return Ok(());
    }

    if keys.is_empty() {
        crate::ui::step("No API keys found");
        return Ok(());
    }

    let id_width = keys
        .iter()
        .map(|k| k.id.to_string().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let app_width = keys.iter().map(|k| k.app.len()).max().unwrap_or(3).max(3);
    let kind_width = keys.iter().map(|k| k.kind.len()).max().unwrap_or(4).max(4);
    let prefix_width = keys
        .iter()
        .map(|k| k.prefix.len())
        .max()
        .unwrap_or(6)
        .max(6);

    println!(
        "{:<id_width$}  {:<app_width$}  {:<kind_width$}  {:<prefix_width$}  {:<8}  {:<24}  CREATED",
        "ID", "APP", "KIND", "PREFIX", "STATUS", "QUOTA"
    );

    for k in &keys {
        let status = if k.revoked_at.is_some() {
            "revoked"
        } else {
            "active"
        };
        let quota = if k.daily_quota > 0 {
            format!("{}/day ({} used)", k.daily_quota, k.used_today)
        } else {
            "unlimited".to_string()
        };
        println!(
            "{:<id_width$}  {:<app_width$}  {:<kind_width$}  {:<prefix_width$}  {:<8}  {:<24}  {}",
            k.id, k.app, k.kind, k.prefix, status, quota, k.created_at
        );
    }

    Ok(())
}

pub async fn create(
    app: &str,
    public: bool,
    origins: &[String],
    quota: Option<i32>,
    json: bool,
) -> Result<()> {
    if app.trim().is_empty() {
        bail!("--app is required");
    }

    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;

    let kind = if public { "public" } else { "secret" };
    let req = CreateKeyRequest {
        app: app.to_string(),
        kind: kind.to_string(),
        allowed_origins: origins.to_vec(),
        daily_quota: quota.unwrap_or(0),
    };

    let resp = client.create_key(&req).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    crate::ui::success(&format!(
        "Created {} API key #{} for {}",
        resp.key.kind, resp.key.id, resp.key.app
    ));
    println!("{}", resp.token);
    crate::ui::hint("Save this token now, it will not be shown again");
    Ok(())
}

pub async fn revoke(id: i64, yes: bool, json: bool) -> Result<()> {
    if id <= 0 {
        bail!("key id must be a positive integer");
    }

    if !yes && io::stdin().is_terminal() {
        print!("Revoke API key #{}? [y/N] ", id);
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if !trimmed.eq_ignore_ascii_case("y") && !trimmed.eq_ignore_ascii_case("yes") {
            crate::ui::step("Revocation aborted");
            return Ok(());
        }
    }

    let config = Config::load()?;
    let client = authed_client(&config, auth::get_token(&config.server_url)?)?;
    client.revoke_key(id).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": id,
                "revoked": true
            }))?
        );
        return Ok(());
    }

    crate::ui::success(&format!("Revoked key {}", id));
    Ok(())
}
