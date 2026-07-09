use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE: &str = "clef";

pub fn store_token(server_url: &str, token: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, server_url)?;
    entry
        .set_password(token)
        .context("failed to store token in keychain")?;
    Ok(())
}

pub fn get_token(server_url: &str) -> Result<Option<String>> {
    let entry = Entry::new(SERVICE, server_url)?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("failed to read token from keychain"),
    }
}

pub fn delete_token(server_url: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, server_url)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("failed to delete token from keychain"),
    }
}
