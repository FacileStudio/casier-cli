use anyhow::{anyhow, bail, Context, Result};
use std::io::{self, Write};
use std::time::Duration;

use crate::api::{ApiClient, AuthConfigResponse};
use crate::auth;
use crate::config::{default_server_url, normalize_server_url, Config};
use crate::loopback;

const SSO_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn run(server: Option<String>, no_browser: bool) -> Result<()> {
    let (server_url, auth_config) = discover_api(resolve_server(server)?).await?;
    let client = ApiClient::new(&server_url, None);

    let token = if auth_config.sso_only {
        if !auth_config.oidc_enabled {
            bail!(
                "{} only allows SSO logins but has no OIDC provider configured",
                server_url
            );
        }
        sso_login(&server_url, no_browser).await?
    } else if auth_config.oidc_enabled && prompt_yes_no("Login with SSO? [Y/n] ")? {
        sso_login(&server_url, no_browser).await?
    } else {
        password_login(&client).await?
    };

    auth::store_token(&server_url, &token)?;
    Config {
        server_url: server_url.clone(),
    }
    .save()?;

    let me = ApiClient::new(&server_url, Some(token)).me().await?;
    crate::ui::success(&format!("Logged in as {} at {}", me.email, server_url));
    Ok(())
}

async fn discover_api(server_url: String) -> Result<(String, AuthConfigResponse)> {
    let mut candidates = vec![server_url.clone()];
    if !server_url.ends_with("/api") {
        candidates.push(format!("{}/api", server_url));
    }

    let mut failure = None;
    for candidate in candidates {
        match ApiClient::new(&candidate, None).auth_config().await {
            Ok(config) => return Ok((candidate, config)),
            Err(err) => failure = Some(err),
        }
    }

    Err(failure.unwrap_or_else(|| anyhow!("no server URL to try"))).with_context(|| {
        format!(
            "no Casier API found at {} (also tried {}/api)\nPoint the CLI at your server with: casier login --server <url>",
            server_url, server_url
        )
    })
}

fn resolve_server(flag: Option<String>) -> Result<String> {
    if let Some(raw) = flag {
        return normalize_server_url(&raw);
    }
    if let Ok(raw) = std::env::var("CASIER_SERVER_URL") {
        return normalize_server_url(&raw);
    }
    if let Some(stored) = Config::stored() {
        return normalize_server_url(&stored.server_url);
    }

    let fallback = default_server_url();
    print!("Casier server URL [{}]: ", fallback);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        Ok(fallback)
    } else {
        normalize_server_url(input)
    }
}

async fn sso_login(server_url: &str, no_browser: bool) -> Result<String> {
    let (listener, port) = loopback::listen().await?;
    let state = loopback::random_state();
    let url = format!(
        "{}/auth/oidc?cli_port={}&cli_state={}",
        server_url, port, state
    );

    if no_browser || !loopback::open_browser(&url) {
        println!("Open this URL to sign in:\n  {}", url);
    } else {
        println!("Opening your browser to sign in…");
        println!("If nothing opened, use:\n  {}", url);
    }
    println!("Waiting for sign-in to complete…");

    loopback::wait_for_token(listener, &state, SSO_TIMEOUT).await
}

async fn password_login(client: &ApiClient) -> Result<String> {
    print!("Email: ");
    io::stdout().flush()?;
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    let email = email.trim().to_string();
    if email.is_empty() {
        bail!("email is required");
    }

    print!("Password: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    Ok(client.login(&email, &password).await?.token)
}

fn prompt_yes_no(question: &str) -> Result<bool> {
    print!("{}", question);
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}
