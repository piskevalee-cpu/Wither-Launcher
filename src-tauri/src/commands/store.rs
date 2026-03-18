// src-tauri/src/commands/store.rs
// Module 11 — Steam Store Commands

use crate::AppState;
use crate::steam::store_api::{SteamStoreApi, AppDetails, FeaturedItem, SearchResults};
use log::info;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreConfig {
    pub cc: String,
    pub l: String,
}

/// Get country code and language from settings
fn get_store_config(state: &State<'_, AppState>) -> StoreConfig {
    let db = state.db.lock().unwrap();
    let conn = db.get_connection();
    
    let cc: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'store_country_code'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "us".to_string());
    
    let l: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'store_language'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "english".to_string());
    
    StoreConfig { cc, l }
}

/// Cache helper
fn get_cached<T: serde::de::DeserializeOwned>(
    conn: &rusqlite::Connection,
    key: &str,
    ttl_secs: u64,
) -> Option<T> {
    let result: Option<(String, i64)> = conn
        .query_row(
            "SELECT value, cached_at FROM store_cache WHERE key = ?",
            params![key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    
    if let Some((value, cached_at)) = result {
        let now = chrono::Utc::now().timestamp() as u64;
        let age = now.saturating_sub(cached_at as u64);
        if age < ttl_secs {
            return serde_json::from_str(&value).ok();
        }
    }
    None
}

fn cache_value(
    conn: &rusqlite::Connection,
    key: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp();
    
    conn.execute(
        "INSERT OR REPLACE INTO store_cache (key, value, cached_at) VALUES (?, ?, ?)",
        params![key, json, now],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn store_get_featured(state: State<'_, AppState>) -> Result<Vec<FeaturedItem>, String> {
    let config = get_store_config(&state);
    let api = SteamStoreApi::new(&config.cc, &config.l);
    
    // Try cache first (1 hour TTL)
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        if let Some(cached) = get_cached::<Vec<FeaturedItem>>(&conn, &format!("featured:{}", config.cc), 3600) {
            info!("Returning cached featured games");
            return Ok(cached);
        }
    }
    
    // Fetch from API
    info!("Fetching featured games from Steam Store API");
    let result = api.get_featured().await?;
    
    // Cache the result
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        let json = serde_json::to_value(&result).map_err(|e| e.to_string())?;
        let _ = cache_value(&conn, &format!("featured:{}", config.cc), &json);
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn store_get_categories(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = get_store_config(&state);
    let api = SteamStoreApi::new(&config.cc, &config.l);
    
    // Try cache first (1 hour TTL)
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        if let Some(cached) = get_cached::<serde_json::Value>(&conn, &format!("categories:{}", config.cc), 3600) {
            info!("Returning cached categories");
            return Ok(cached);
        }
    }
    
    // Fetch from API
    info!("Fetching featured categories from Steam Store API");
    let result = api.get_featured_categories().await?;
    
    // Cache the result
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        let _ = cache_value(&conn, &format!("categories:{}", config.cc), &result);
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn store_get_app(state: State<'_, AppState>, app_id: u32) -> Result<AppDetails, String> {
    let config = get_store_config(&state);
    let api = SteamStoreApi::new(&config.cc, &config.l);
    
    // Try cache first (24 hours TTL)
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        if let Some(cached) = get_cached::<AppDetails>(&conn, &format!("appdetails:{}:{}", app_id, config.cc), 86400) {
            info!("Returning cached app details for {}", app_id);
            return Ok(cached);
        }
    }
    
    // Fetch from API
    info!("Fetching app details for {}", app_id);
    let result = api.get_app_details(app_id).await?;
    
    // Cache the result
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        let json = serde_json::to_value(&result).map_err(|e| e.to_string())?;
        let _ = cache_value(&conn, &format!("appdetails:{}:{}", app_id, config.cc), &json);
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn store_search(
    state: State<'_, AppState>,
    query: String,
    page: u32,
) -> Result<SearchResults, String> {
    let config = get_store_config(&state);
    let api = SteamStoreApi::new(&config.cc, &config.l);
    
    // Search results are cached for 15 minutes (in-memory only, not persisted)
    // For simplicity, we just fetch from API
    
    info!("Searching for: '{}' (page {})", query, page);
    let result = api.search(&query, page, 20).await?;
    
    Ok(result)
}

#[tauri::command]
pub async fn store_browse(
    state: State<'_, AppState>,
    filter: String,
    page: u32,
) -> Result<SearchResults, String> {
    let config = get_store_config(&state);
    let api = SteamStoreApi::new(&config.cc, &config.l);
    
    info!("Browsing filter: '{}' (page {})", filter, page);
    let result = api.browse(&filter, page, 20).await?;
    
    Ok(result)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    info!("Opening URL in browser: {}", url);
    open::that(&url).map_err(|e| format!("Failed to open URL: {}", e))
}
