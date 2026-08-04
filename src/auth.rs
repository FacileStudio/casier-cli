use anyhow::{bail, Context, Result};
use keyring::Entry;

const SERVICE: &str = "casier";

pub fn store_token(server_url: &str, token: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, server_url)?;
    entry
        .set_password(token)
        .context("failed to store token in keychain")?;

    match entry.get_password() {
        Ok(stored) if stored == token => Ok(()),
        _ => bail!(
            "the token did not survive being written to the keychain\n\
             Set CASIER_TOKEN in your environment instead"
        ),
    }
}

pub fn get_token(server_url: &str) -> Result<Option<String>> {
    if let Ok(token) = std::env::var("CASIER_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Some(token.to_string()));
        }
    }

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
