use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;

pub struct ApiClient {
    base_url: String,
    token: Option<String>,
    client: Client,
}

#[derive(Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Deserialize)]
struct ExchangeResponse {
    pub token: String,
}

#[derive(Deserialize)]
pub struct AuthConfigResponse {
    #[serde(default)]
    pub sso_only: bool,
    #[serde(default)]
    pub oidc_enabled: bool,
}

#[derive(Deserialize)]
pub struct MeResponse {
    pub email: String,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub description: String,
}

/// A secret as the list route returns it. `value` is absent unless the request
/// asked to reveal it, so it is an `Option` here rather than a `String`: a
/// missing value must never silently decode as an empty one.
#[derive(Deserialize)]
pub struct Secret {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub version: i32,
}

/// A secret that is known to carry its value. Every command that injects,
/// writes or compares values takes this type, so the "did the server actually
/// send values?" question is answered once, at the boundary, instead of at each
/// call site.
#[derive(Debug)]
pub struct RevealedSecret {
    pub key: String,
    pub value: String,
    pub version: i32,
}

/// The server answered a revealing read with metadata alone.
///
/// It is a distinct type, not a plain message, because it must not be mistaken
/// for a server that could merely not be reached: falling back to a cached copy
/// is right for that and wrong for this.
#[derive(Debug)]
pub struct MissingValues {
    pub missing: usize,
    pub total: usize,
}

impl std::fmt::Display for MissingValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the server returned {} of {} secrets without values — this casier CLI is older than the server it is talking to. Reinstall it, then retry.",
            self.missing, self.total
        )
    }
}

impl std::error::Error for MissingValues {}

/// Converts a list that was requested with `reveal=true`, failing loudly if the
/// server answered with metadata alone.
///
/// That happens when this binary is older than the server it talks to: it does
/// not know to ask for values, and every value comes back absent. Injecting
/// that into a child process, an `.env` file or a deployment would produce an
/// environment with nothing in it and no error — the command runs, appears to
/// work, and connects to nothing.
fn into_revealed(secrets: Vec<Secret>) -> Result<Vec<RevealedSecret>> {
    let missing = secrets.iter().filter(|s| s.value.is_none()).count();
    if missing > 0 {
        return Err(MissingValues {
            missing,
            total: secrets.len(),
        }
        .into());
    }
    Ok(secrets
        .into_iter()
        .map(|s| RevealedSecret {
            key: s.key,
            value: s.value.unwrap_or_default(),
            version: s.version,
        })
        .collect())
}

#[derive(Deserialize)]
struct ProjectsResponse {
    #[serde(default)]
    projects: Option<Vec<Project>>,
}

#[derive(Deserialize)]
struct SecretsResponse {
    #[serde(default)]
    secrets: Option<Vec<Secret>>,
}

#[derive(Deserialize)]
pub struct ImportResponse {
    pub created: i32,
    pub updated: i32,
    pub skipped: i32,
}

#[derive(Deserialize)]
pub struct ExportResponse {
    pub content: String,
}

impl ApiClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client: Client::new(),
        }
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    pub async fn auth_config(&self) -> Result<AuthConfigResponse> {
        let resp = self
            .request(reqwest::Method::GET, "/auth/config")
            .send()
            .await
            .with_context(|| format!("failed to reach {}", self.base_url))?;
        if !resp.status().is_success() {
            bail!(
                "GET {}/auth/config returned {}",
                self.base_url,
                resp.status()
            );
        }
        let body = resp.text().await?;
        serde_json::from_str(&body).with_context(|| {
            format!(
                "{} did not answer with a Casier API response",
                self.base_url
            )
        })
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthResponse> {
        let resp = self
            .request(reqwest::Method::POST, "/auth/login")
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("login failed ({}): {}", status, body);
        }
        Ok(resp.json().await?)
    }

    /// Exchanges a one-time porte login code for a bearer token. The code only
    /// works once and expires in under a minute, so it is fetched and consumed
    /// in the same sign-in run.
    pub async fn exchange(&self, code: &str) -> Result<String> {
        let resp = self
            .request(reqwest::Method::POST, "/auth/oidc/exchange")
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("exchanging the sign-in code failed ({}): {}", status, body);
        }
        let exchanged: ExchangeResponse = resp.json().await?;
        Ok(exchanged.token)
    }

    pub async fn me(&self) -> Result<MeResponse> {
        let resp = self
            .request(reqwest::Method::GET, "/auth/me")
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            bail!("GET /auth/me returned {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        let resp = self
            .request(reqwest::Method::GET, "/projects")
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            bail!("GET /projects returned {}", resp.status());
        }
        let body: ProjectsResponse = resp.json().await?;
        Ok(body.projects.unwrap_or_default())
    }

    /// Lists keys, versions and tags without values. Use this whenever the
    /// command only needs to know which secrets exist.
    pub async fn list_secrets(&self, project: &str, env: &str) -> Result<Vec<Secret>> {
        self.fetch_secrets(project, env, false).await
    }

    /// Lists secrets with their values, which the server audits apart from a
    /// metadata listing and records by count.
    pub async fn reveal_secrets(&self, project: &str, env: &str) -> Result<Vec<RevealedSecret>> {
        into_revealed(self.fetch_secrets(project, env, true).await?)
    }

    async fn fetch_secrets(&self, project: &str, env: &str, reveal: bool) -> Result<Vec<Secret>> {
        let path = format!(
            "/projects/{}/environments/{}/secrets{}",
            project,
            env,
            if reveal { "?reveal=true" } else { "" }
        );
        let resp = self
            .request(reqwest::Method::GET, &path)
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            bail!("GET {} returned {}", path, resp.status());
        }
        let body: SecretsResponse = resp.json().await?;
        Ok(body.secrets.unwrap_or_default())
    }

    pub async fn set_secret(
        &self,
        project: &str,
        env: &str,
        key: &str,
        value: &str,
    ) -> Result<Secret> {
        let path = format!("/projects/{}/environments/{}/secrets", project, env);
        let resp = self
            .request(reqwest::Method::POST, &path)
            .json(&serde_json::json!({ "key": key, "value": value }))
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("POST {} failed ({}): {}", path, status, body);
        }
        Ok(resp.json().await?)
    }

    /// Reads one secret through the per-key reveal route, which the server
    /// audits by key rather than folding into a bulk read.
    pub async fn reveal_secret(
        &self,
        project: &str,
        env: &str,
        key: &str,
    ) -> Result<RevealedSecret> {
        let path = format!(
            "/projects/{}/environments/{}/secrets/{}/reveal",
            project, env, key
        );
        let resp = self
            .request(reqwest::Method::GET, &path)
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            bail!("GET {} returned {}", path, resp.status());
        }
        let secret: Secret = resp.json().await?;
        into_revealed(vec![secret])?
            .pop()
            .context(format!("secret '{}' not found", key))
    }

    pub async fn import_env(
        &self,
        project: &str,
        env: &str,
        content: &str,
    ) -> Result<ImportResponse> {
        let path = format!("/projects/{}/environments/{}/secrets/import", project, env);
        let resp = self
            .request(reqwest::Method::POST, &path)
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("POST {} failed ({}): {}", path, status, body);
        }
        Ok(resp.json().await?)
    }

    pub async fn export_env(&self, project: &str, env: &str) -> Result<ExportResponse> {
        let path = format!("/projects/{}/environments/{}/secrets/export", project, env);
        let resp = self
            .request(reqwest::Method::GET, &path)
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            bail!("GET {} returned {}", path, resp.status());
        }
        Ok(resp.json().await?)
    }

    pub async fn delete_secret(&self, project: &str, env: &str, key: &str) -> Result<()> {
        let path = format!("/projects/{}/environments/{}/secrets/{}", project, env, key);
        let resp = self
            .request(reqwest::Method::DELETE, &path)
            .send()
            .await
            .context("failed to reach server")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("DELETE {} failed ({}): {}", path, status, body);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Vec<Secret> {
        let parsed: SecretsResponse = serde_json::from_str(body).unwrap();
        parsed.secrets.unwrap_or_default()
    }

    #[test]
    fn revealed_list_keeps_its_values() {
        let secrets = into_revealed(parse(
            r#"{"secrets":[{"key":"A","value":"1","version":3}]}"#,
        ))
        .unwrap();
        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].key, "A");
        assert_eq!(secrets[0].value, "1");
        assert_eq!(secrets[0].version, 3);
    }

    #[test]
    fn an_empty_value_is_a_real_value() {
        let secrets = into_revealed(parse(r#"{"secrets":[{"key":"A","value":""}]}"#)).unwrap();
        assert_eq!(secrets[0].value, "");
    }

    #[test]
    fn an_empty_environment_is_not_an_error() {
        assert!(into_revealed(parse(r#"{"secrets":[]}"#))
            .unwrap()
            .is_empty());
    }

    /// A server newer than this binary answers a plain list with metadata only.
    /// Refusing here is the whole point: the alternative is a child process
    /// launched with an empty environment and a zero exit code.
    #[test]
    fn metadata_without_values_is_refused() {
        let err = into_revealed(parse(
            r#"{"secrets":[{"key":"A","version":1},{"key":"B","version":1}]}"#,
        ))
        .unwrap_err();
        let message = format!("{}", err);
        assert!(message.contains("without values"), "{}", message);
        assert!(message.contains("older than the server"), "{}", message);
    }

    #[test]
    fn a_partially_valueless_list_is_refused() {
        let err = into_revealed(parse(
            r#"{"secrets":[{"key":"A","value":"1"},{"key":"B"}]}"#,
        ))
        .unwrap_err();
        assert!(format!("{}", err).contains("1 of 2"));
    }
}
