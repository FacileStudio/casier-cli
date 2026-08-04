use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

const CASIER_TOML: &str = ".casier.toml";

pub async fn run() -> Result<()> {
    let target = PathBuf::from(CASIER_TOML);
    if target.exists() {
        bail!("{} already exists in this directory", CASIER_TOML);
    }

    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let spaces = client.list_spaces().await?;

    if spaces.is_empty() {
        bail!("No spaces found. Create one in the dashboard first.");
    }

    println!("Available spaces:");
    for (i, s) in spaces.iter().enumerate() {
        println!("  [{}] {} ({})", i + 1, s.name, s.slug);
    }

    print!("\nSelect a space [1-{}]: ", spaces.len());
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice: usize = input
        .trim()
        .parse()
        .context("invalid selection")?;
    if choice < 1 || choice > spaces.len() {
        bail!("Selection out of range");
    }
    let space = &spaces[choice - 1];

    print!("Default environment [dev]: ");
    io::stdout().flush()?;
    let mut env_input = String::new();
    io::stdin().read_line(&mut env_input)?;
    let env = env_input.trim();
    let env = if env.is_empty() { "dev" } else { env };

    let content = format!(
        r#"[project]
space = "{}"
environment = "{}"
"#,
        space.slug, env
    );

    std::fs::write(&target, &content)
        .with_context(|| format!("failed to write {}", CASIER_TOML))?;

    println!("Created {} (space={}, env={})", CASIER_TOML, space.slug, env);
    Ok(())
}
