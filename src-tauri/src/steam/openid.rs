// src-tauri/src/steam/openid.rs
// Steam OpenID 2.0 Authentication Module

use log::{error, info};
use reqwest::Client;
use std::collections::HashMap;
use tiny_http::{Header, Response, Server};
use urlencoding::encode;

const STEAM_OPENID_ENDPOINT: &str = "https://steamcommunity.com/openid/login";
const LOCAL_CALLBACK_PORT: u16 = 14069;

#[derive(Debug, Clone)]
pub struct SteamUser {
    pub steamid: String,
    pub personaname: String,
    pub avatar: String,
    pub avatarfull: String,
    pub profileurl: String,
    pub is_public: bool,
}

#[derive(Debug)]
pub enum AuthError {
    Cancelled,
    InvalidSignature,
    MissingClaimedId,
    InvalidClaimedId,
    Timeout,
    PortInUse,
    ServerError,
    NetworkError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Cancelled => write!(f, "Authentication cancelled"),
            AuthError::InvalidSignature => write!(f, "Invalid signature from Steam"),
            AuthError::MissingClaimedId => write!(f, "Missing claimed ID"),
            AuthError::InvalidClaimedId => write!(f, "Invalid claimed ID format"),
            AuthError::Timeout => write!(f, "Authentication timeout (2 minutes)"),
            AuthError::PortInUse => write!(f, "Port 14069 is already in use"),
            AuthError::ServerError => write!(f, "Local server error"),
            AuthError::NetworkError(e) => write!(f, "Network error: {}", e),
        }
    }
}

/// Build the Steam OpenID redirect URL
pub fn build_auth_url() -> String {
    let callback = format!("http://localhost:{}/auth/steam/callback", LOCAL_CALLBACK_PORT);
    let realm = format!("http://localhost:{}", LOCAL_CALLBACK_PORT);

    let params = [
        ("openid.ns", "http://specs.openid.net/auth/2.0"),
        ("openid.mode", "checkid_setup"),
        ("openid.return_to", &callback),
        ("openid.realm", &realm),
        ("openid.identity", "http://specs.openid.net/auth/2.0/identifier_select"),
        ("openid.claimed_id", "http://specs.openid.net/auth/2.0/identifier_select"),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{}?{}", STEAM_OPENID_ENDPOINT, query)
}

/// Parse query string from URL
fn parse_query(url: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    if let Some(query) = url.split('?').nth(1) {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    key.to_string(),
                    urlencoding::decode(value).unwrap_or_else(|_| value.into()).to_string(),
                );
            }
        }
    }
    params
}

/// Verify the OpenID response with Steam
async fn verify_with_steam(params: &HashMap<String, String>) -> Result<bool, AuthError> {
    let client = Client::new();

    // Build verification params
    let mut verify_params = params.clone();
    verify_params.insert("openid.mode".to_string(), "check_authentication".to_string());

    let response = client
        .post(STEAM_OPENID_ENDPOINT)
        .form(&verify_params)
        .send()
        .await
        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

    let text = response
        .text()
        .await
        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

    // Steam returns "is_valid:true" if the authentication is valid
    Ok(text.contains("is_valid:true"))
}

/// Extract SteamID64 from claimed_id
fn extract_steamid64(claimed_id: &str) -> Result<u64, AuthError> {
    // Format: https://steamcommunity.com/openid/id/76561198XXXXXXXXX
    claimed_id
        .split('/')
        .last()
        .ok_or(AuthError::InvalidClaimedId)?
        .parse()
        .map_err(|_| AuthError::InvalidClaimedId)
}

/// Handle the callback from Steam
async fn handle_callback(params: HashMap<String, String>) -> Result<u64, AuthError> {
    // Check openid.mode == "id_res"
    if params.get("openid.mode") != Some(&"id_res".to_string()) {
        return Err(AuthError::Cancelled);
    }

    // Verify with Steam
    if !verify_with_steam(&params).await? {
        return Err(AuthError::InvalidSignature);
    }

    // Extract SteamID64 from claimed_id
    let claimed_id = params
        .get("openid.claimed_id")
        .ok_or(AuthError::MissingClaimedId)?;

    extract_steamid64(claimed_id)
}

/// Run the complete authentication flow
pub async fn run_auth_flow() -> Result<u64, AuthError> {
    // Try ports 14069-14079
    let mut server = None;
    let mut port = LOCAL_CALLBACK_PORT;

    for try_port in LOCAL_CALLBACK_PORT..=LOCAL_CALLBACK_PORT + 10 {
        match Server::http(format!("127.0.0.1:{}", try_port)) {
            Ok(s) => {
                server = Some(s);
                port = try_port;
                break;
            }
            Err(_) => continue,
        }
    }

    let server = server.ok_or(AuthError::PortInUse)?;

    info!("Local auth server started on port {}", port);

    // Build and open auth URL in browser
    let auth_url = if port == LOCAL_CALLBACK_PORT {
        build_auth_url()
    } else {
        // Rebuild URL with actual port
        let callback = format!("http://localhost:{}/auth/steam/callback", port);
        let realm = format!("http://localhost:{}", port);

        let params = [
            ("openid.ns", "http://specs.openid.net/auth/2.0"),
            ("openid.mode", "checkid_setup"),
            ("openid.return_to", &callback),
            ("openid.realm", &realm),
            ("openid.identity", "http://specs.openid.net/auth/2.0/identifier_select"),
            ("openid.claimed_id", "http://specs.openid.net/auth/2.0/identifier_select"),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", STEAM_OPENID_ENDPOINT, query)
    };

    // Open browser
    if let Err(e) = open::that(&auth_url) {
        error!("Failed to open browser: {}", e);
        return Err(AuthError::NetworkError(e.to_string()));
    }

    // Wait for callback with timeout (120 seconds)
    let request_result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::task::spawn_blocking(move || server.recv()),
    )
    .await;

    let request = match request_result {
        Ok(Ok(Ok(req))) => req,
        Ok(Ok(Err(_))) => return Err(AuthError::ServerError),
        Ok(Err(_)) => return Err(AuthError::Cancelled),
        Err(_) => return Err(AuthError::Timeout),
    };

    // Parse query params from request URL
    let params = parse_query(request.url());

    // Send 200 OK response
    let response = Response::from_string(
        r#"<!DOCTYPE html>
<html>
<head><title>Wither - Steam Login</title></head>
<body style="font-family: sans-serif; text-align: center; padding: 50px;">
    <h2>✅ Logged in successfully!</h2>
    <p>You can close this tab and return to Wither.</p>
</body>
</html>"#,
    )
    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());

    let _ = request.respond(response);

    // Verify and return SteamID64
    handle_callback(params).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_auth_url() {
        let url = build_auth_url();
        assert!(url.starts_with(STEAM_OPENID_ENDPOINT));
        assert!(url.contains("openid.ns"));
        assert!(url.contains("openid.mode=checkid_setup"));
    }

    #[test]
    fn test_parse_query() {
        let url = "http://localhost:14069/callback?foo=bar&baz=qux";
        let params = parse_query(url);
        assert_eq!(params.get("foo"), Some(&"bar".to_string()));
        assert_eq!(params.get("baz"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_extract_steamid64() {
        let claimed_id = "https://steamcommunity.com/openid/id/76561198012345678";
        let steamid = extract_steamid64(claimed_id).unwrap();
        assert_eq!(steamid, 76561198012345678);
    }
}
