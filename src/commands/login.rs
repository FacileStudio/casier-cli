use anyhow::{bail, Result};
use std::io::{self, Write};

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

pub async fn run() -> Result<()> {
    let config = Config::load()?;
    let client = ApiClient::new(&config.server_url, None);

    let auth_config = match client.auth_config().await {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Could not fetch auth config, falling back to password login.");
            crate::api::AuthConfigResponse {
                sso_only: false,
                oidc_enabled: false,
            }
        }
    };

    if auth_config.sso_only {
        bail!(
            "This server requires SSO login.\nOpen your browser to: {}/auth/oidc",
            config.server_url
        );
    }

    if auth_config.oidc_enabled {
        print!("Login with SSO? [Y/n] ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_lowercase();

        if answer.is_empty() || answer == "y" || answer == "yes" {
            let url = format!("{}/auth/oidc", config.server_url);
            println!("Open this URL in your browser:\n  {}", url);

            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&url).spawn();
            }

            print!("\nPaste your token here: ");
            io::stdout().flush()?;
            let token = rpassword::read_password()?;
            if token.is_empty() {
                bail!("No token provided.");
            }

            auth::store_token(&config.server_url, &token)?;

            let authed_client = ApiClient::new(&config.server_url, Some(token));
            let me = authed_client.me().await?;
            println!("Logged in as {}", me.email);
            return Ok(());
        }
    }

    print!("Email: ");
    io::stdout().flush()?;
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    let email = email.trim().to_string();

    print!("Password: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    let resp = client.login(&email, &password).await?;
    auth::store_token(&config.server_url, &resp.token)?;

    let authed_client = ApiClient::new(&config.server_url, Some(resp.token));
    let me = authed_client.me().await?;
    println!("Logged in as {}", me.email);
    Ok(())
}
