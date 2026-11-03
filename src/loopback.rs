use anyhow::{anyhow, Context, Result};
use std::io::Read;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_REQUEST_LINE: usize = 8192;

const PAGE_HEAD: &str = "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Casier</title>\
<style>:root{color-scheme:light dark;--bg:#fafafa;--fg:#18181b;--muted:#71717a;--card:#fff;--warn:#b45309}\
@media (prefers-color-scheme:dark){:root{--bg:#09090b;--fg:#fafafa;--muted:#a1a1aa;--card:#18181b;--warn:#fbbf24}}\
*{box-sizing:border-box}\
body{margin:0;min-height:100dvh;display:flex;align-items:center;justify-content:center;padding:1.5rem;\
background:var(--bg);color:var(--fg);font:16px/1.6 ui-sans-serif,system-ui,-apple-system,'Segoe UI',Roboto,sans-serif}\
main{width:100%;max-width:26rem}\
.brand{text-align:center;margin-bottom:2rem;font-size:1.125rem;font-weight:650;letter-spacing:-.01em}\
.card{background:var(--card);border-radius:.75rem;padding:1.75rem;text-align:center}\
h1{margin:0;font-size:1.25rem;font-weight:650;letter-spacing:-.01em}\
p{margin:.5rem 0 0;font-size:.875rem;color:var(--muted)}\
p.warn{color:var(--warn)}\
.hint{margin-top:1.5rem;text-align:center;font-size:.8125rem;color:var(--muted)}</style>";

fn page(heading: &str, body: &str, hint: &str, warn: bool) -> String {
    let class = if warn { " class=\"warn\"" } else { "" };
    format!(
        "{PAGE_HEAD}<main><div class=\"brand\">Casier</div><div class=\"card\"><h1>{heading}</h1>\
<p{class}>{body}</p></div><p class=\"hint\">{hint}</p></main>"
    )
}

fn success_page() -> String {
    page(
        "Signed in",
        "Casier has your login.",
        "You can close this tab and go back to your terminal.",
        false,
    )
}

fn failure_page() -> String {
    page(
        "Sign-in refused",
        "The callback did not match this login attempt.",
        "The login started in your terminal is still waiting.",
        true,
    )
}

pub async fn listen() -> Result<(TcpListener, u16)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("cannot open a local callback port")?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

pub async fn wait_for_code(
    listener: TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<String> {
    tokio::time::timeout(timeout, accept_callback(listener, expected_state))
        .await
        .map_err(|_| anyhow!("timed out waiting for the browser to complete sign-in"))?
}

async fn accept_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let request_line = match read_request_line(&mut stream).await {
            Ok(line) => line,
            Err(_) => continue,
        };

        match parse_callback(&request_line) {
            Some((code, state)) if state == expected_state => {
                respond(&mut stream, "200 OK", &success_page()).await.ok();
                return Ok(code);
            }
            Some(_) => {
                respond(&mut stream, "400 Bad Request", &failure_page())
                    .await
                    .ok();
            }
            None => {
                respond(&mut stream, "404 Not Found", "").await.ok();
            }
        }
    }
}

async fn read_request_line(stream: &mut TcpStream) -> Result<String> {
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];
    while buffer.len() < MAX_REQUEST_LINE {
        let read = stream.read(&mut byte).await?;
        if read == 0 {
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        if byte[0] != b'\r' {
            buffer.push(byte[0]);
        }
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

fn parse_callback(request_line: &str) -> Option<(String, String)> {
    let target = request_line.split_whitespace().nth(1)?;
    let (path, query) = target.split_once('?')?;
    if path != "/" && path != "/callback" {
        return None;
    }

    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        match key {
            "code" => code = Some(percent_decode(value)),
            "state" => state = Some(percent_decode(value)),
            _ => {}
        }
    }

    Some((code?, state?))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 3 <= bytes.len() => {
                let decoded = std::str::from_utf8(&bytes[index + 1..index + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                match decoded {
                    Some(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    None => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn random_state() -> String {
    let mut bytes = [0u8; 16];
    let from_urandom = std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .is_ok();

    if !from_urandom {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mixed = nanos ^ ((std::process::id() as u128) << 96);
        bytes.copy_from_slice(&mixed.to_le_bytes());
    }

    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

pub fn open_browser(url: &str) -> bool {
    let spawned = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    spawned.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_code_and_state_from_root_redirect() {
        let line = "GET /?code=abc-123_XY&state=deadbeef HTTP/1.1";
        let (code, state) = parse_callback(line).unwrap();
        assert_eq!(code, "abc-123_XY");
        assert_eq!(state, "deadbeef");
    }

    #[test]
    fn accepts_the_legacy_callback_path() {
        let line = "GET /callback?code=abc&state=deadbeef HTTP/1.1";
        let (code, state) = parse_callback(line).unwrap();
        assert_eq!(code, "abc");
        assert_eq!(state, "deadbeef");
    }

    #[test]
    fn decodes_escaped_values() {
        let line = "GET /?code=a%2Fb%26c&state=x+y HTTP/1.1";
        let (code, state) = parse_callback(line).unwrap();
        assert_eq!(code, "a/b&c");
        assert_eq!(state, "x y");
    }

    #[test]
    fn ignores_unrelated_requests() {
        assert!(parse_callback("GET /favicon.ico HTTP/1.1").is_none());
        assert!(parse_callback("GET /?code=only HTTP/1.1").is_none());
        assert!(parse_callback("garbage").is_none());
    }

    #[test]
    fn random_state_is_unique_and_url_safe() {
        let a = random_state();
        let b = random_state();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
