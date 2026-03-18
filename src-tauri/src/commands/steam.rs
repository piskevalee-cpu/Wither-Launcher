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
pub fn sync_steam(state: State<'_, AppState>) -> Result<SyncResult, String> {
    // This is a simplified version - full implementation in steam module
    crate::steam::sync_steam_library(&state)
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
    use std::path::PathBuf;
    
    let mut debug_info = serde_json::json!({
        "steam_root": null,
        "library_folders": [],
        "acf_files": []
    });

    // Get Steam root from registry
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                debug_info["steam_root"] = serde_json::json!(path);
                
                let vdf_path = PathBuf::from(&path).join("steamapps").join("libraryfolders.vdf");
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
        }
    }

    Ok(debug_info)
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
    use std::path::PathBuf;
    
    // Try to find the ACF file
    let steam_path = PathBuf::from(r"C:\Program Files (x86)\Steam");
    let acf_path = steam_path.join("steamapps").join(format!("appmanifest_{}.acf", app_id));
    
    if !acf_path.exists() {
        return Err(format!("ACF file not found: {:?}", acf_path));
    }
    
    std::fs::read_to_string(&acf_path)
        .map_err(|e| format!("Failed to read file: {}", e))
}
