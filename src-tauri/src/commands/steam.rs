use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncResult {
    pub added: i64,
    pub updated: i64,
    pub removed: i64,
    pub errors: Vec<String>,
    pub synced_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SteamUser {
    pub steamid: String,
    pub personaname: String,
    pub avatar: String,
    pub avatarfull: String,
    pub profileurl: String,
    pub is_public: bool,
}

/// Auto-detect logged-in Steam user from local Steam config (loginusers.vdf)
/// Works cross-platform: reads from the appropriate Steam config directory
#[tauri::command]
pub async fn auto_detect_steam_user(state: State<'_, AppState>) -> Result<Option<SteamUser>, String> {
    use log::info;
    
    let steam_root = get_steam_root_for_debug();
    let steam_root = match steam_root {
        Some(p) => p,
        None => {
            info!("Steam root not found for auto-detect");
            return Ok(None);
        }
    };

    let loginusers_path = steam_root.join("config").join("loginusers.vdf");
    if !loginusers_path.exists() {
        info!("loginusers.vdf not found at {:?}", loginusers_path);
        return Ok(None);
    }

    let content = std::fs::read_to_string(&loginusers_path)
        .map_err(|e| format!("Failed to read loginusers.vdf: {}", e))?;

    // Parse the VDF to find the most recent user (MostRecent = 1)
    let (steamid, personaname) = match parse_loginusers_vdf(&content) {
        Some(result) => result,
        None => {
            info!("No logged-in user found in loginusers.vdf");
            return Ok(None);
        }
    };

    info!("Auto-detected Steam user: {} ({})", personaname, steamid);

    // Try to fetch avatar if API key is available
    let api_key: String = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'steam_api_key'",
            [],
            |row| row.get(0),
        ).unwrap_or_default()
    };

    let (avatar, avatarfull) = if !api_key.is_empty() {
        match crate::steam::web_api::fetch_player_summary(&api_key, &steamid).await {
            Ok(player) => (player.avatar, player.avatarfull),
            Err(_) => (String::new(), String::new()),
        }
    } else {
        (String::new(), String::new())
    };

    let user = SteamUser {
        steamid: steamid.clone(),
        personaname: personaname.clone(),
        avatar: avatar.clone(),
        avatarfull: avatarfull.clone(),
        profileurl: format!("https://steamcommunity.com/profiles/{}", steamid),
        is_public: true,
    };

    // Save to settings
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        
        let _ = conn.execute(
            "UPDATE settings SET value = ? WHERE key = 'steam_user_id'",
            [steamid.as_str()],
        );
        let _ = conn.execute(
            "UPDATE settings SET value = ? WHERE key = 'steam_username'",
            [personaname.as_str()],
        );
        let _ = conn.execute(
            "UPDATE settings SET value = ? WHERE key = 'steam_avatar_url'",
            [avatar.as_str()],
        );
    }

    Ok(Some(user))
}

/// Parse loginusers.vdf to find the most recently logged-in user
fn parse_loginusers_vdf(content: &str) -> Option<(String, String)> {
    let mut current_steamid = String::new();
    let mut current_persona = String::new();
    let mut current_most_recent = false;
    let mut best_steamid = String::new();
    let mut best_persona = String::new();
    let mut best_timestamp: u64 = 0;
    let mut in_user_block = false;
    let mut depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed == "{" {
            depth += 1;
            continue;
        }
        if trimmed == "}" {
            if depth == 2 && in_user_block {
                // End of a user block
                if current_most_recent {
                    return Some((current_steamid.clone(), current_persona.clone()));
                }
                // Track as fallback by timestamp
                in_user_block = false;
                current_most_recent = false;
            }
            depth -= 1;
            continue;
        }

        if depth == 1 {
            // Top level under "users" — this is a SteamID key
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 2 {
                current_steamid = parts[1].to_string();
                current_persona = String::new();
                current_most_recent = false;
                in_user_block = true;
            }
        }

        if depth == 2 && in_user_block {
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 4 {
                let key = parts[1].to_lowercase();
                let value = parts[3].to_string();

                match key.as_str() {
                    "personaname" => current_persona = value,
                    "mostrecent" => current_most_recent = value == "1",
                    "timestamp" => {
                        if let Ok(ts) = value.parse::<u64>() {
                            if ts > best_timestamp {
                                best_timestamp = ts;
                                best_steamid = current_steamid.clone();
                                best_persona = current_persona.clone();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // If no MostRecent=1 found, use the one with highest timestamp
    if !best_steamid.is_empty() {
        Some((best_steamid, best_persona))
    } else {
        None
    }
}

#[tauri::command]
pub async fn steam_login(state: State<'_, AppState>) -> Result<SteamUser, String> {
    use crate::steam::openid::run_auth_flow;
    use crate::steam::web_api::fetch_player_summary;
    
    // Run OpenID flow to get SteamID64
    let steam_id = run_auth_flow()
        .await
        .map_err(|e| format!("Steam login failed: {}", e))?;

    // Get API key from settings (optional - will work without it but no avatar)
    let api_key: String = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'steam_api_key'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default()
    }; // DB lock released here

    // Fetch player summary if API key available
    let (personaname, avatar, avatarfull) = if !api_key.is_empty() {
        match fetch_player_summary(&api_key, &steam_id.to_string()).await {
            Ok(player) => (player.personaname, player.avatar, player.avatarfull),
            Err(_) => (format!("SteamUser_{}", steam_id), String::new(), String::new()),
        }
    } else {
        (format!("SteamUser_{}", steam_id), String::new(), String::new())
    };

    // Save to settings (reacquire lock)
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        
        conn.execute(
            "UPDATE settings SET value = ? WHERE key = 'steam_user_id'",
            [steam_id.to_string()],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE settings SET value = ? WHERE key = 'steam_username'",
            [personaname.as_str()],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE settings SET value = ? WHERE key = 'steam_avatar_url'",
            [avatar.as_str()],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(SteamUser {
        steamid: steam_id.to_string(),
        personaname,
        avatar,
        avatarfull,
        profileurl: format!("https://steamcommunity.com/profiles/{}", steam_id),
        is_public: true,
    })
}

#[tauri::command]
pub fn steam_logout(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    // Clear Steam credentials from settings
    conn.execute(
        "UPDATE settings SET value = '' WHERE key IN ('steam_user_id', 'steam_username', 'steam_avatar_url')",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_steam_user(state: State<'_, AppState>) -> Result<Option<SteamUser>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    // Get saved Steam user info from settings
    let steam_id: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'steam_user_id'",
            [],
            |row| row.get(0),
        )
        .ok();

    if let Some(steamid) = steam_id {
        let personaname: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'steam_username'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| String::new());

        let avatar: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'steam_avatar_url'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| String::new());

        Ok(Some(SteamUser {
            steamid: steamid.clone(),
            personaname,
            avatar: avatar.clone(),
            avatarfull: avatar.clone(),
            profileurl: format!("https://steamcommunity.com/profiles/{}", steamid),
            is_public: true,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn sync_steam(state: State<'_, AppState>) -> Result<SyncResult, String> {
    // Run sync in a blocking thread to avoid freezing the UI
    let state_clone = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        // Create a temporary State-like wrapper
        crate::steam::sync_steam_library_with_db(&state_clone.db)
    })
    .await
    .map_err(|e| format!("Sync task failed: {}", e))?
}

#[tauri::command]
pub fn get_steam_games(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare("SELECT * FROM games WHERE source = 'steam'")
        .map_err(|e| e.to_string())?;

    let games = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "source": row.get::<_, String>(2)?,
                "steam_app_id": row.get::<_, Option<i64>>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for game in games {
        result.push(game.map_err(|e| e.to_string())?);
    }

    Ok(result)
}

#[tauri::command]
pub fn debug_steam_paths() -> Result<serde_json::Value, String> {
    use crate::steam::acf_parser::parse_library_folders;
    
    let mut debug_info = serde_json::json!({
        "steam_root": null,
        "library_folders": [],
        "acf_files": []
    });

    // Get Steam root path (cross-platform)
    let steam_root = get_steam_root_for_debug();

    if let Some(ref root_path) = steam_root {
        debug_info["steam_root"] = serde_json::json!(root_path.to_string_lossy().to_string());

        let vdf_path = root_path.join("steamapps").join("libraryfolders.vdf");
        if vdf_path.exists() {
            if let Ok(paths) = parse_library_folders(&vdf_path) {
                debug_info["library_folders"] = serde_json::json!(
                    paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
                );

                // Scan for .acf files
                let mut acf_files = Vec::new();
                for steam_path in &paths {
                    let steamapps = steam_path.join("steamapps");
                    if steamapps.exists() {
                        if let Ok(entries) = std::fs::read_dir(&steamapps) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().map_or(false, |ext| ext == "acf") {
                                    acf_files.push(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
                debug_info["acf_files"] = serde_json::json!(acf_files);
            }
        }
    }

    Ok(debug_info)
}

/// Cross-platform Steam root detection for debug command
fn get_steam_root_for_debug() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                return Some(PathBuf::from(path));
            }
        }
        
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                return Some(PathBuf::from(path));
            }
        }

        let default = PathBuf::from(r"C:\Program Files (x86)\Steam");
        if default.exists() {
            return Some(default);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
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
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(format!("{}/Library/Application Support/Steam", home));
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

#[tauri::command]
pub fn reset_steam_games(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    // Count games to be deleted
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM games WHERE source = 'steam'",
        [],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // Delete all Steam games
    conn.execute(
        "DELETE FROM games WHERE source = 'steam'",
        [],
    ).map_err(|e| e.to_string())?;

    Ok(count)
}

#[tauri::command]
pub fn clear_removed_steam_games(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    // Count removed games
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM steam_removed_games",
        [],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // Clear the tracking table
    conn.execute("DELETE FROM steam_removed_games", []).map_err(|e| e.to_string())?;

    Ok(count)
}

#[tauri::command]
pub fn read_acf_file(app_id: u32) -> Result<String, String> {
    // Use cross-platform Steam root detection
    let steam_root = get_steam_root_for_debug()
        .ok_or("Steam installation not found")?;

    let acf_path = steam_root.join("steamapps").join(format!("appmanifest_{}.acf", app_id));
    
    if !acf_path.exists() {
        // Also check additional library folders
        let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
        if vdf_path.exists() {
            if let Ok(paths) = crate::steam::acf_parser::parse_library_folders(&vdf_path) {
                for lib_path in paths {
                    let alt_acf = lib_path.join("steamapps").join(format!("appmanifest_{}.acf", app_id));
                    if alt_acf.exists() {
                        return std::fs::read_to_string(&alt_acf)
                            .map_err(|e| format!("Failed to read file: {}", e));
                    }
                }
            }
        }
        return Err(format!("ACF file not found for AppID {}", app_id));
    }
    
    std::fs::read_to_string(&acf_path)
        .map_err(|e| format!("Failed to read file: {}", e))
}
