// src-tauri/src/steam/web_api.rs
// Steam Web API Client Module

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const API_BASE: &str = "https://api.steampowered.com";
const STORE_BASE: &str = "https://store.steampowered.com/api";

#[derive(Debug, Deserialize, Clone)]
pub struct AppDetails {
    pub name: String,
    pub header_image: Option<String>,
    pub background: Option<String>,
    pub genres: Option<Vec<Genre>>,
    pub developers: Option<Vec<String>>,
    pub release_date: Option<ReleaseDate>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Genre {
    pub description: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseDate {
    pub date: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlayerSummary {
    pub steamid: String,
    pub personaname: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    pub profileurl: String,
    pub communityvisibilitystate: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OwnedGame {
    pub appid: u32,
    pub name: String,
    pub playtime_forever: u64,
    pub playtime_2weeks: Option<u64>,
    pub img_icon_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OwnedGamesResponse {
    pub game_count: u32,
    pub games: Vec<OwnedGame>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecentlyPlayedGame {
    pub appid: u32,
    pub name: String,
    pub playtime_2weeks: u64,
    pub playtime_forever: u64,
    pub img_icon_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecentlyPlayedGamesResponse {
    pub total_count: u32,
    pub games: Vec<RecentlyPlayedGame>,
}

/// Fetch player summary (profile info)
pub async fn fetch_player_summary(api_key: &str, steam_id: &str) -> Result<PlayerSummary, String> {
    let client = Client::new();
    let url = format!(
        "{}/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
        API_BASE, api_key, steam_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let players = json["response"]["players"]
        .as_array()
        .ok_or("No players in response")?;

    if players.is_empty() {
        return Err("Player not found".to_string());
    }

    serde_json::from_value(players[0].clone())
        .map_err(|e| format!("Failed to parse player: {}", e))
}

/// Fetch owned games list
pub async fn fetch_owned_games(api_key: &str, steam_id: &str) -> Result<OwnedGamesResponse, String> {
    let client = Client::new();
    // Use the correct URL format with key as query parameter
    let url = format!(
        "{}/IPlayerService/GetOwnedGames/v1/?key={}&steamid={}&include_appinfo=1&include_played_free_games=1&format=json",
        API_BASE, api_key, steam_id
    );

    println!("Fetching from URL: {}", url);

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    println!("API Status: {}", status);

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    println!("API Response (first 500 chars): {}", &text[..text.len().min(500)]);

    // Check if we got HTML instead of JSON (error page)
    if text.trim().starts_with("<!DOCTYPE") || text.trim().starts_with("<html") {
        return Err("Steam API returned HTML error page. Please check your API key at https://steamcommunity.com/dev/apikey".to_string());
    }

    // Check for common error responses
    if text.contains("\"error\"") || text.contains("\"errordesc\"") || text.contains("Unauthorized") {
        return Err(format!("Steam API error: {}", text));
    }

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}. Response: {}", e, &text[..text.len().min(200)]))?;

    // Check for privacy error or empty library
    if json["response"]["game_count"].as_u64() == Some(0) {
        return Err("Library is private or empty".to_string());
    }

    serde_json::from_value(json["response"].clone())
        .map_err(|e| format!("Failed to parse games: {}", e))
}

/// Fetch recently played games
pub async fn fetch_recently_played_games(api_key: &str, steam_id: &str) -> Result<RecentlyPlayedGamesResponse, String> {
    let client = Client::new();
    let url = format!(
        "{}/IPlayerService/GetRecentlyPlayedGames/v1/?key={}&steamid={}",
        API_BASE, api_key, steam_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    serde_json::from_value(json["response"].clone())
        .map_err(|e| format!("Failed to parse games: {}", e))
}

/// Fetch app details from Store API (no key required)
pub async fn fetch_app_details(app_id: u32) -> Result<Option<AppDetails>, String> {
    let client = Client::new();
    let url = format!(
        "{}/appdetails?appids={}",
        STORE_BASE, app_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let json: HashMap<String, serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if let Some(app_data) = json.get(&app_id.to_string()) {
        if app_data["success"].as_bool() == Some(true) {
            let details: AppDetails = serde_json::from_value(app_data["data"].clone())
                .map_err(|e| format!("Failed to parse details: {}", e))?;
            return Ok(Some(details));
        }
    }

    Ok(None)
}

/// Get cover URL for a game
pub fn get_cover_url(app_id: u32) -> String {
    format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_600x900.jpg",
        app_id
    )
}

/// Get background URL for a game
pub fn get_background_url(app_id: u32) -> String {
    format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_hero.jpg",
        app_id
    )
}

/// Get icon URL for a game
pub fn get_icon_url(app_id: u32, icon_hash: &str) -> String {
    format!(
        "https://media.steampowered.com/steamcommunity/public/images/apps/{}/{}.jpg",
        app_id, icon_hash
    )
}
