use anyhow::{bail, Result};
use std::collections::BTreeMap;

use crate::api::ApiClient;
use crate::auth;
use crate::cache;
use crate::config::Config;

pub async fn run(space: &str, env_a: &str, env_b: &str) -> Result<()> {
    let config = Config::load()?;
    let token = auth::get_token(&config.server_url)?;
    let Some(token) = token else {
        bail!("Not logged in. Run `casier login` first.");
    };

    let client = ApiClient::new(&config.server_url, Some(token));
    let (secrets_a, secrets_b) = tokio::try_join!(
        client.list_secrets(space, env_a),
        client.list_secrets(space, env_b),
    )?;

    cache::store(space, env_a, &cache::to_map(&secrets_a));
    cache::store(space, env_b, &cache::to_map(&secrets_b));

    let map_a: BTreeMap<&str, &str> = secrets_a.iter().map(|s| (s.key.as_str(), s.value.as_str())).collect();
    let map_b: BTreeMap<&str, &str> = secrets_b.iter().map(|s| (s.key.as_str(), s.value.as_str())).collect();

    let mut has_diff = false;

    for (key, val_a) in &map_a {
        match map_b.get(key) {
            Some(val_b) if val_a != val_b => {
                println!("~ {}  ({} → {})", key, mask(val_a), mask(val_b));
                has_diff = true;
            }
            None => {
                println!("- {}  (only in {})", key, env_a);
                has_diff = true;
            }
            _ => {}
        }
    }

    for key in map_b.keys() {
        if !map_a.contains_key(key) {
            println!("+ {}  (only in {})", key, env_b);
            has_diff = true;
        }
    }

    if !has_diff {
        println!("No differences between {} and {}.", env_a, env_b);
    }

    Ok(())
}

fn mask(value: &str) -> String {
    if value.len() <= 4 {
        "*".repeat(value.len().max(3))
    } else {
        format!("{}{}{}",
            &value[..2],
            "*".repeat(value.len().saturating_sub(4).min(8)),
            &value[value.len()-2..],
        )
    }
}
