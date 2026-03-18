pub mod acf_parser;
pub mod web_api;
pub mod openid;
pub mod store_api;

use crate::AppState;
use crate::steam::acf_parser::{scan_all_libraries, parse_library_folders};
use crate::steam::web_api::{fetch_owned_games, fetch_app_details, get_cover_url, get_background_url};
use chrono::Utc;
use log::{info, warn, error};
use rusqlite::params;
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;

use crate::commands::steam::SyncResult;

pub fn sync_steam_library(state: &tauri::State<'_, AppState>) -> Result<SyncResult, String> {
    let mut added = 0;
    let mut updated = 0;
    let mut removed = 0;
    let mut errors = Vec::new();

    // Get API key and SteamID from settings
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();
    
    let api_key: String = conn
        .query_row("SELECT value FROM settings WHERE key = 'steam_api_key'", [], |row| row.get(0))
        .unwrap_or_default();
    
    let steam_id: String = conn
        .query_row("SELECT value FROM settings WHERE key = 'steam_user_id'", [], |row| row.get(0))
        .unwrap_or_default();

    drop(db); // Release lock before async operations

    info!("=== STEAM SYNC STARTED ===");
    info!("API Key present: {}", !api_key.is_empty());
    info!("Steam ID: {}", if steam_id.is_empty() { "none" } else { &steam_id });

    // Get all Steam library paths
    let steam_paths = get_all_steam_paths();

    if steam_paths.is_empty() {
        warn!("No Steam installation found");
        return Ok(SyncResult {
            added: 0,
            updated: 0,
            removed: 0,
            errors: vec!["Steam installation not found".to_string()],
            synced_at: Utc::now().timestamp(),
        });
    }

    info!("Found {} Steam library paths", steam_paths.len());
    for path in &steam_paths {
        info!("  - {:?}", path);
    }

    // Scan all libraries for installed games
    let acf_games = scan_all_libraries(&steam_paths);
    info!("Found {} installed games from .acf files", acf_games.len());
    for game in &acf_games {
        info!("  - {} (AppID: {})", game.name, game.app_id);
    }

    // Get owned games from Web API if credentials available
    let mut api_game_ids = Vec::new();
    if !api_key.is_empty() && !steam_id.is_empty() {
        info!("Fetching owned games from Steam API...");
        match tokio::runtime::Runtime::new()
            .map_err(|e| e.to_string())?
            .block_on(fetch_owned_games(&api_key, &steam_id))
        {
            Ok(owned) => {
                info!("Fetched {} owned games from API", owned.games.len());
                api_game_ids = owned.games.iter().map(|g| g.appid).collect();
            }
            Err(e) => {
                warn!("Failed to fetch owned games: {}", e);
                errors.push(format!("API fetch failed: {}", e));
            }
        }
    } else {
        info!("Skipping API fetch - missing credentials");
    }

    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection_mut();
    let now = Utc::now().timestamp();

    // Track which games we've seen
    let mut seen_app_ids = Vec::new();

    // Get list of manually removed Steam games
    let removed_app_ids: Vec<i64> = conn
        .prepare("SELECT steam_app_id FROM steam_removed_games")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    info!("Found {} manually removed Steam games", removed_app_ids.len());

    // Process installed games from .acf files
    for acf_game in &acf_games {
        // Skip manually removed games
        if removed_app_ids.contains(&(acf_game.app_id as i64)) {
            info!("Skipping removed game: {} (AppID: {})", acf_game.name, acf_game.app_id);
            continue;
        }
        
        seen_app_ids.push(acf_game.app_id);
        let game_id = format!("steam_{}", acf_game.app_id);

        // Check if game exists in DB
        let exists: bool = conn
            .query_row("SELECT 1 FROM games WHERE id = ?", params![game_id], |row| row.get(0))
            .unwrap_or(0) != 0;

        // Fetch metadata from API if available
        let (cover_url, background_url, genre, developer) = 
            fetch_game_metadata(acf_game.app_id);

        if exists {
            // ALWAYS update is_installed based on ACF StateFlags
            let is_installed_flag = if acf_game.state_flags == 4 || acf_game.state_flags == 6 { 1 } else { 0 };
            log::info!("Updating game {} - StateFlags: {}, is_installed: {}", game_id, acf_game.state_flags, is_installed_flag);
            
            // Force update is_installed even if game exists
            let rows_affected = conn.execute(
                "UPDATE games SET 
                    name = ?, 
                    is_installed = ?,
                    cover_url = ?,
                    background_url = ?,
                    genre = ?,
                    developer = ?,
                    last_synced_at = ?
                WHERE id = ?",
                params![
                    acf_game.name,
                    is_installed_flag,  // Force update this!
                    cover_url,
                    background_url,
                    genre,
                    developer,
                    now,
                    game_id
                ],
            ).map_err(|e| {
                error!("Failed to update game {}: {}", game_id, e);
                e.to_string()
            })?;
            
            info!("Updated game {} (rows: {})", game_id, rows_affected);
            updated += 1;
        } else {
            // Insert new game
            let is_installed_flag = if acf_game.state_flags == 4 || acf_game.state_flags == 6 { 1 } else { 0 };
            log::info!("Inserting game {} - StateFlags: {}, is_installed: {}", game_id, acf_game.state_flags, is_installed_flag);
            
            let rows_affected = conn.execute(
                "INSERT INTO games (
                    id, name, source, drm_type, launch_method,
                    steam_app_id, is_installed, cover_url, background_url,
                    genre, developer, added_at, last_synced_at
                ) VALUES (?, ?, 'steam', 'steam', 'steam_protocol', ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    game_id,
                    acf_game.name,
                    acf_game.app_id as i64,
                    if acf_game.state_flags == 4 || acf_game.state_flags == 6 { 1 } else { 0 },
                    cover_url,
                    background_url,
                    genre,
                    developer,
                    now,
                    now
                ],
            ).map_err(|e| {
                error!("Failed to insert game {}: {}", game_id, e);
                e.to_string()
            })?;
            
            info!("Inserted game {} (rows: {})", game_id, rows_affected);
            added += 1;
        }

        // Rate limit: 300ms delay between API calls
        std::thread::sleep(Duration::from_millis(300));
    }

    // Add uninstalled owned games from API
    for app_id in &api_game_ids {
        // Skip manually removed games
        if removed_app_ids.contains(&(*app_id as i64)) {
            info!("Skipping removed API game: AppID {}", app_id);
            continue;
        }
        
        if !seen_app_ids.contains(app_id) {
            let game_id = format!("steam_{}", app_id);
            
            // Check if already in DB
            let exists: bool = conn
                .query_row("SELECT 1 FROM games WHERE id = ?", params![game_id], |row| row.get(0))
                .unwrap_or(0) != 0;

            if !exists {
                // Fetch metadata including name from Store API
                let (cover_url, background_url, genre, developer, game_name) = fetch_game_metadata_with_name(*app_id);

                let rows_affected = conn.execute(
                    "INSERT INTO games (
                        id, name, source, drm_type, launch_method,
                        steam_app_id, is_installed, cover_url, background_url,
                        genre, developer, added_at, last_synced_at
                    ) VALUES (?, ?, 'steam', 'steam', 'steam_protocol', ?, 0, ?, ?, ?, ?, ?, ?)",
                    params![
                        game_id,
                        game_name,
                        *app_id as i64,
                        cover_url,
                        background_url,
                        genre,
                        developer,
                        now,
                        now
                    ],
                ).map_err(|e| {
                    error!("Failed to insert API game {}: {}", game_id, e);
                    e.to_string()
                })?;
                
                info!("Inserted API game {} (rows: {})", game_id, rows_affected);
                added += 1;
            }

            seen_app_ids.push(*app_id);
            
            // Rate limit
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    // Mark games as uninstalled if not in local libraries
    let steam_games: Vec<i64> = conn
        .prepare("SELECT steam_app_id FROM games WHERE source = 'steam'")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    info!("Found {} total Steam games in database", steam_games.len());

    for app_id in steam_games {
        if !seen_app_ids.contains(&(app_id as u32)) {
            let _ = conn.execute(
                "UPDATE games SET is_installed = 0 WHERE steam_app_id = ?",
                params![app_id],
            );
            removed += 1;
        }
    }

    info!("=== STEAM SYNC COMPLETE ===");
    info!("Added: {}, Updated: {}, Removed: {}", added, updated, removed);

    Ok(SyncResult {
        added,
        updated,
        removed,
        errors,
        synced_at: Utc::now().timestamp(),
    })
}

/// Fetch metadata for a game from Steam Web API
fn fetch_game_metadata(app_id: u32) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return (None, None, None, None),
    };

    // Fetch app details
    match runtime.block_on(fetch_app_details(app_id)) {
        Ok(Some(details)) => {
            let cover = Some(get_cover_url(app_id));
            let background = Some(get_background_url(app_id));
            let genre = details.genres
                .and_then(|g| g.first().map(|g| g.description.clone()));
            let developer = details.developers
                .and_then(|d| d.first().cloned());
            (cover, background, genre, developer)
        }
        _ => {
            // Fallback to CDN URLs
            (Some(get_cover_url(app_id)), Some(get_background_url(app_id)), None, None)
        }
    }
}

/// Fetch metadata including game name from Steam Store API
fn fetch_game_metadata_with_name(app_id: u32) -> (Option<String>, Option<String>, Option<String>, Option<String>, String) {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return (None, None, None, None, format!("Game {}", app_id)),
    };

    // Fetch app details from Store API
    match runtime.block_on(fetch_app_details(app_id)) {
        Ok(Some(details)) => {
            let cover = Some(get_cover_url(app_id));
            let background = Some(get_background_url(app_id));
            let genre = details.genres
                .and_then(|g| g.first().map(|g| g.description.clone()));
            let developer = details.developers
                .and_then(|d| d.first().cloned());
            let name = details.name;
            (cover, background, genre, developer, name)
        }
        _ => {
            // Fallback to CDN URLs and placeholder name
            (Some(get_cover_url(app_id)), Some(get_background_url(app_id)), None, None, format!("Game {}", app_id))
        }
    }
}

/// Get all Steam library paths from registry and libraryfolders.vdf
fn get_all_steam_paths() -> Vec<PathBuf> {
    let mut all_paths = Vec::new();

    // Get main Steam path
    if let Some(steam_root) = get_steam_root() {
        // Parse libraryfolders.vdf for additional libraries
        let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
        if vdf_path.exists() {
            if let Ok(mut library_paths) = parse_library_folders(&vdf_path) {
                all_paths.append(&mut library_paths);
            }
        }
        
        // Always include the main Steam path
        if !all_paths.contains(&steam_root) {
            all_paths.push(steam_root);
        }
    }

    all_paths
}

/// Get the main Steam installation path
fn get_steam_root() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        
        // Try WOW6432Node first (64-bit Windows)
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                return Some(PathBuf::from(path));
            }
        }
        
        // Try regular key (32-bit Windows)
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                return Some(PathBuf::from(path));
            }
        }

        // Fallback to default path
        let default = PathBuf::from(r"C:\Program Files (x86)\Steam");
        if default.exists() {
            return Some(default);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").ok()?;
        let path = PathBuf::from(format!("{}/Library/Application Support/Steam", home));
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").ok()?;
        
        // Try multiple locations
        let candidates = vec![
            format!("{}/.steam/steam", home),
            format!("{}/.local/share/Steam", home),
        ];

        for path_str in candidates {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

pub fn background_sync(_app_handle: &AppHandle) {
    // Module 10: Sync only runs once at startup, no periodic sync
    // Manual sync is triggered by user via UI
    info!("Background sync disabled - sync only runs at startup");
}
