use anyhow::{bail, Result};

use crate::api::ApiClient;
use crate::auth;
use crate::config::Config;

pub async fn run() -> Result<()> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let spaces = client.list_spaces().await?;

    if spaces.is_empty() {
        println!("No spaces found.");
        return Ok(());
    }

    let name_width = spaces.iter().map(|s| s.name.len()).max().unwrap_or(0);
    let slug_width = spaces.iter().map(|s| s.slug.len()).max().unwrap_or(0);

    println!(
        "{:<name_width$}  {:<slug_width$}  {}",
        "NAME", "SLUG", "DESCRIPTION"
    );

    for s in &spaces {
        println!(
            "{:<name_width$}  {:<slug_width$}  {}",
            s.name, s.slug, s.description
        );
    }
    Ok(())
}
