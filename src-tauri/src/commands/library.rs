use crate::AppState;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub source: String,
    pub drm_type: String,
    pub launch_method: String,
    pub executable_path: Option<String>,
    pub launch_args: Option<String>,
    pub steam_app_id: Option<i64>,
    pub cover_url: Option<String>,
    pub background_url: Option<String>,
    pub genre: Option<String>,
    pub developer: Option<String>,
    pub release_year: Option<i64>,
    pub steam_playtime_s: Option<i64>,
    pub is_installed: bool,
    pub is_favourite: bool,
    pub added_at: i64,
    pub last_synced_at: Option<i64>,
    pub wither_playtime_s: i64,
    pub last_played_at: i64,
    pub session_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct AddGamePayload {
    pub name: Option<String>,
    pub executable_path: String,
    pub cover_path: Option<String>,
    pub launch_args: Option<Vec<String>>,
}

#[tauri::command]
pub fn get_all_games(state: State<'_, AppState>) -> Result<Vec<Game>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare("SELECT * FROM v_games_full ORDER BY last_played_at DESC")
        .map_err(|e| e.to_string())?;

    let games = stmt
        .query_map([], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                drm_type: row.get(3)?,
                launch_method: row.get(4)?,
                executable_path: row.get(5)?,
                launch_args: row.get(6)?,
                steam_app_id: row.get(7)?,
                cover_url: row.get(8)?,
                background_url: row.get(9)?,
                genre: row.get(10)?,
                developer: row.get(11)?,
                release_year: row.get(12)?,
                steam_playtime_s: row.get(13)?,
                is_installed: row.get::<_, i64>(14)? != 0,
                is_favourite: row.get::<_, i64>(15)? != 0,
                added_at: row.get(16)?,
                last_synced_at: row.get(17)?,
                wither_playtime_s: row.get(18)?,
                last_played_at: row.get(19)?,
                session_count: row.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for game in games {
        result.push(game.map_err(|e| e.to_string())?);
    }

    Ok(result)
}

#[tauri::command]
pub fn add_custom_game(
    state: State<'_, AppState>,
    executable_path: String,
    name: Option<String>,
    launch_args: Option<Vec<String>>,
) -> Result<Game, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection_mut();

    let game_id = format!("custom_{}", Uuid::new_v4());
    let now = Utc::now().timestamp();

    // Infer name from executable path if not provided
    let game_name = name.unwrap_or_else(|| {
        infer_name_from_path(&executable_path)
    });

    let launch_args_json = launch_args
        .map(|args| serde_json::to_string(&args).unwrap_or_default());

    conn.execute(
        "INSERT INTO games (
            id, name, source, drm_type, launch_method, 
            executable_path, launch_args, added_at
        ) VALUES (?, ?, 'custom', 'none', 'executable', ?, ?, ?)",
        params![
            game_id,
            game_name,
            executable_path,
            launch_args_json,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    // Fetch the inserted game
    let mut stmt = conn
        .prepare("SELECT * FROM v_games_full WHERE id = ?")
        .map_err(|e| e.to_string())?;

    let game = stmt
        .query_row(params![game_id], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                drm_type: row.get(3)?,
                launch_method: row.get(4)?,
                executable_path: row.get(5)?,
                launch_args: row.get(6)?,
                steam_app_id: row.get(7)?,
                cover_url: row.get(8)?,
                background_url: row.get(9)?,
                genre: row.get(10)?,
                developer: row.get(11)?,
                release_year: row.get(12)?,
                steam_playtime_s: row.get(13)?,
                is_installed: row.get::<_, i64>(14)? != 0,
                is_favourite: row.get::<_, i64>(15)? != 0,
                added_at: row.get(16)?,
                last_synced_at: row.get(17)?,
                wither_playtime_s: row.get(18)?,
                last_played_at: row.get(19)?,
                session_count: row.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(game)
}

#[tauri::command]
pub fn remove_game(state: State<'_, AppState>, game_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    // Check if this is a Steam game
    let steam_app_id: Option<i64> = conn
        .query_row(
            "SELECT steam_app_id FROM games WHERE id = ?",
            params![game_id],
            |row| row.get(0),
        )
        .ok();

    // Delete the game
    conn.execute("DELETE FROM games WHERE id = ?", params![game_id])
        .map_err(|e| e.to_string())?;

    // If it's a Steam game, add to removed tracking table
    if let Some(app_id) = steam_app_id {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO steam_removed_games (steam_app_id, removed_at, reason) VALUES (?, ?, 'manual')",
            params![app_id, now],
        )
        .map_err(|e| e.to_string())?;
        
        log::info!("Marked Steam AppID {} as manually removed", app_id);
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpdateGamePayload {
    pub game_id: String,
    pub name: String,
    pub executable_path: String,
    pub cover_url: Option<String>,
}

#[tauri::command]
pub fn update_custom_game(
    state: State<'_, AppState>,
    game_id: String,
    name: String,
    executable_path: String,
    cover_url: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE games SET name = ?, executable_path = ?, cover_url = ? WHERE id = ?",
        params![name, executable_path, cover_url, game_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub fn update_game_last_played(
    state: State<'_, AppState>,
    game_id: String,
    last_played_at: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    conn.execute(
        "UPDATE games SET last_played_at = ? WHERE id = ?",
        params![last_played_at, game_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let value: String = conn
        .query_row("SELECT value FROM settings WHERE key = ?", params![key], |row| row.get(0))
        .unwrap_or_default();

    Ok(value)
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = ?",
        params![key, value, value],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn infer_name_from_path(path: &str) -> String {
    use std::path::Path;
    
    let path = Path::new(path);
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    // Replace separators with spaces
    let spaced = stem.replace(['_', '-', '.'], " ");

    // Title case each word
    spaced
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
