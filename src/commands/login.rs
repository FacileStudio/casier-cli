use anyhow::{anyhow, bail, Context, Result};
use std::io::IsTerminal;
use std::io::{self, Write};
use std::time::Duration;

use crate::api::{ApiClient, AuthConfigResponse};
use crate::auth;
use crate::config::{default_server_url, normalize_server_url, Config};
use crate::device;
use crate::loopback;

const SSO_TIMEOUT: Duration = Duration::from_secs(300);

/// Signs in and stores the token. With SSO the default is the device grant:
/// an approval link any browser can open, so it works over SSH. `listener`
/// opts into the loopback redirect, which is also the fallback for a server
/// that has not mounted the device exchange. Without SSO it asks for a password.
pub async fn run(server: Option<String>, no_browser: bool, listener: bool) -> Result<()> {
    let (server_url, auth_config) = discover_api(resolve_server(server)?).await?;
    let client = ApiClient::new(&server_url, None);

    if auth_config.sso_only && !auth_config.oidc_enabled {
        bail!(
            "{} only allows SSO logins but has no OIDC provider configured",
            server_url
        );
    }
    let token = if !auth_config.oidc_enabled {
        password_login(&client).await?
    } else if listener || !device::exchange_mounted(&server_url).await? {
        if !listener {
            crate::ui::warn(
                "This server has no device sign-in yet, falling back to the browser redirect.",
            );
        }
        sso_login(&server_url, no_browser).await?
    } else {
        let opens = !no_browser && browser_available();
        device::exchange(&server_url, &device::access_token(opens).await?).await?
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

    normalize_server_url(&default_server_url())
}

fn browser_available() -> bool {
    if !io::stdout().is_terminal() {
        return false;
    }
    if cfg!(target_os = "linux") {
        return std::env::var_os("DISPLAY").is_some()
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
    }
    true
}

async fn sso_login(server_url: &str, no_browser: bool) -> Result<String> {
    let (listener, port) = loopback::listen().await?;
    let state = loopback::random_state();
    let url = format!(
        "{}/auth/oidc?flow=cli&port={}&cli_state={}",
        server_url, port, state
    );

    if no_browser || !loopback::open_browser(&url) {
        println!("Open this URL to sign in:\n  {}", url);
        println!("Waiting for sign-in to complete…");
    } else {
        println!("Waiting for the browser to complete sign-in…");
    }

    let code = loopback::wait_for_code(listener, &state, SSO_TIMEOUT).await?;
    ApiClient::new(server_url, None).exchange(&code).await
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
