use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

use crate::config::PROJECT_CONFIG_NAMES;

pub async fn run() -> Result<()> {
    let target = PathBuf::from(PROJECT_CONFIG_NAMES[0]);
    for existing in PROJECT_CONFIG_NAMES {
        if PathBuf::from(existing).exists() {
            bail!("{} already exists in this directory", existing);
        }
    }

    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("not logged in — run `casier login`");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let projects = client.list_projects().await?;

    if projects.is_empty() {
        bail!("No projects found. Create one in the dashboard first.");
    }

    println!("Available projects:");
    for (i, s) in projects.iter().enumerate() {
        println!("  [{}] {} ({})", i + 1, s.name, s.slug);
    }

    print!("\nSelect a project [1-{}]: ", projects.len());
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse().context("invalid selection")?;
    if choice < 1 || choice > projects.len() {
        bail!("Selection out of range");
    }
    let project = &projects[choice - 1];

    print!("Default environment [dev]: ");
    io::stdout().flush()?;
    let mut env_input = String::new();
    io::stdin().read_line(&mut env_input)?;
    let env = env_input.trim();
    let env = if env.is_empty() { "dev" } else { env };

    let content = format!(
        "project:\n  slug: {}\n  environment: {}\n",
        project.slug, env
    );

    let name = PROJECT_CONFIG_NAMES[0];
    std::fs::write(&target, &content).with_context(|| format!("failed to write {}", name))?;

    crate::ui::success(&format!(
        "Created {} (project={}, env={})",
        name, project.slug, env
    ));
    Ok(())
}
