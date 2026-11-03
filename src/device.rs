use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::time::{Duration, Instant};

use crate::ui;

/// Registre, the suite's identity provider. Only its root discovery document is
/// real: a per-application path answers 200 with the dashboard's HTML.
const ISSUER: &str = "https://sso.facile.studio";

/// The public client Registre registers for the suite's command lines. A
/// secret compiled into a shipped binary is readable by everyone, so none exists.
const CLIENT_ID: &str = "facile-cli";

const SCOPES: &str = "openid email profile";

/// The grant identifier from RFC 8628 §3.4, spelled exactly as the RFC does.
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// RFC 8628 §3.5: every slow_down adds five seconds, cumulatively.
const SLOW_DOWN: Duration = Duration::from_secs(5);

const TIMEOUT: Duration = Duration::from_secs(15);

const RETRY: &str = "run `casier login` again";

#[derive(Deserialize)]
struct Provider {
    device_authorization_endpoint: Option<String>,
    token_endpoint: String,
    #[serde(default)]
    grant_types_supported: Vec<String>,
}

#[derive(Deserialize)]
struct Authorization {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    interval: Option<u64>,
    expires_in: Option<u64>,
}

#[derive(Deserialize, Default)]
struct Answer {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct Session {
    token: String,
}

#[derive(Debug, PartialEq)]
enum Poll {
    Approved(String),
    Pending,
    SlowDown,
}

/// Whether the server mounts porte's device exchange. porte answers an empty
/// request with 400 when it does and 404 when it does not, a contract kept
/// stable so a client can probe it before showing a code nobody can redeem.
pub async fn exchange_mounted(api: &str) -> Result<bool> {
    let response = reqwest::Client::new()
        .post(format!("{api}/auth/oidc/device/exchange"))
        .timeout(TIMEOUT)
        .json(&serde_json::json!({}))
        .send()
        .await
        .with_context(|| format!("cannot reach {api}"))?;
    Ok(mounted(response.status().as_u16()))
}

fn mounted(status: u16) -> bool {
    status != 404
}

/// Runs the RFC 8628 device grant against Registre and returns its access
/// token. Nothing listens on this machine, so it works over SSH: the approval
/// link can be opened in any browser. The token is only ever handed to
/// [`exchange`], never written to disk.
pub async fn access_token(open_browser: bool) -> Result<String> {
    let provider: Provider = reqwest::Client::new()
        .get(format!("{ISSUER}/.well-known/openid-configuration"))
        .timeout(TIMEOUT)
        .send()
        .await
        .context("cannot reach the identity provider")?
        .json()
        .await
        .context("the identity provider served no OpenID configuration")?;
    let offered = provider
        .grant_types_supported
        .iter()
        .any(|grant| grant == GRANT_TYPE);
    let Some(endpoint) = provider.device_authorization_endpoint.filter(|_| offered) else {
        bail!("{ISSUER} does not offer device sign-in, run `casier login --listener`");
    };

    let auth: Authorization = form(&endpoint, &[("client_id", CLIENT_ID), ("scope", SCOPES)])
        .await
        .context("the identity provider would not start a device sign-in")?;
    let link = auth
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&auth.verification_uri);
    if !link.starts_with("https://") {
        bail!("the identity provider sent an unusable sign-in link: {link}");
    }

    ui::step(&format!(
        "Approve this sign-in at {link} (code {})",
        auth.user_code
    ));
    if open_browser {
        let _ = crate::loopback::open_browser(link);
    }
    poll(&provider.token_endpoint, &auth).await
}

/// Waits for the human, honouring the provider's interval. A transport error
/// is retried rather than fatal, since the deadline already bounds the loop.
async fn poll(token_endpoint: &str, auth: &Authorization) -> Result<String> {
    let mut interval = Duration::from_secs(auth.interval.unwrap_or(5));
    let deadline = Instant::now() + Duration::from_secs(auth.expires_in.unwrap_or(600));
    let fields = [
        ("grant_type", GRANT_TYPE),
        ("device_code", auth.device_code.as_str()),
        ("client_id", CLIENT_ID),
    ];
    while Instant::now() < deadline {
        tokio::time::sleep(interval).await;
        let Ok(answer) = form::<Answer>(token_endpoint, &fields).await else {
            continue;
        };
        match read(answer)? {
            Poll::Approved(token) => return Ok(token),
            Poll::SlowDown => interval += SLOW_DOWN,
            Poll::Pending => {}
        }
    }
    bail!("the sign-in link expired before it was approved, {RETRY}")
}

/// Turns one token-endpoint answer into approved, keep waiting, or stop.
/// RFC 8628 §3.5's errors are not interchangeable: telling somebody their code
/// expired when they refused it sends them to retry what they meant to stop.
fn read(answer: Answer) -> Result<Poll> {
    if let Some(token) = answer.access_token.filter(|token| !token.is_empty()) {
        return Ok(Poll::Approved(token));
    }
    match answer.error.as_deref() {
        Some("authorization_pending") => Ok(Poll::Pending),
        Some("slow_down") => Ok(Poll::SlowDown),
        Some("access_denied") => {
            bail!("the sign-in was refused in the browser, {RETRY} if that was a mistake")
        }
        Some("expired_token") => bail!("the sign-in link expired before it was approved, {RETRY}"),
        other => bail!(
            "the identity provider refused the sign-in: {}",
            answer
                .error_description
                .as_deref()
                .or(other)
                .unwrap_or("no reason given")
        ),
    }
}

async fn form<T: DeserializeOwned>(url: &str, fields: &[(&str, &str)]) -> Result<T> {
    let response = reqwest::Client::new()
        .post(url)
        .timeout(TIMEOUT)
        .form(fields)
        .send()
        .await
        .context("cannot reach the identity provider")?;
    Ok(response.json().await?)
}

/// Trades Registre's access token for a Casier session. The exchange looks the
/// account up and never creates one, so a 401 almost always means this account
/// has never signed in to the web app.
pub async fn exchange(api: &str, access_token: &str) -> Result<String> {
    let response = reqwest::Client::new()
        .post(format!("{api}/auth/oidc/device/exchange"))
        .timeout(TIMEOUT)
        .json(&serde_json::json!({ "access_token": access_token }))
        .send()
        .await
        .context("cannot reach the server to finish the sign-in")?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let site = api.strip_suffix("/api").unwrap_or(api);
        bail!("the server does not know this account yet. Sign in once at {site} in a browser, then {RETRY}.");
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("the server refused the sign-in ({status}): {body}");
    }
    let session: Session = response
        .json()
        .await
        .context("the server's answer to the device exchange was not what was expected")?;
    if session.token.is_empty() {
        bail!("the server returned an empty token");
    }
    Ok(session.token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(error: &str) -> Answer {
        Answer {
            error: Some(error.to_string()),
            ..Answer::default()
        }
    }

    #[test]
    fn polling_waits_slows_down_then_succeeds() {
        assert_eq!(
            read(answer("authorization_pending")).unwrap(),
            Poll::Pending
        );
        assert_eq!(read(answer("slow_down")).unwrap(), Poll::SlowDown);
        let approved = Answer {
            access_token: Some("registre".to_string()),
            ..Answer::default()
        };
        assert_eq!(
            read(approved).unwrap(),
            Poll::Approved("registre".to_string())
        );
    }

    #[test]
    fn polling_stops_on_expiry_denial_and_unknown_errors() {
        assert!(read(answer("expired_token"))
            .unwrap_err()
            .to_string()
            .contains("expired"));
        assert!(read(answer("access_denied"))
            .unwrap_err()
            .to_string()
            .contains("refused"));
        assert!(read(answer("invalid_grant")).is_err());
        assert!(read(Answer::default()).is_err());
    }

    #[test]
    fn only_a_404_means_the_exchange_is_missing() {
        assert!(!mounted(404));
        assert!(mounted(400));
        assert!(mounted(401));
    }
}
