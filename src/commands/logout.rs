use anyhow::Result;

use crate::auth;
use crate::config::Config;

pub fn run() -> Result<()> {
    let config = Config::load()?;
    auth::delete_token(&config.server_url)?;
    crate::ui::success("Logged out");
    Ok(())
}
