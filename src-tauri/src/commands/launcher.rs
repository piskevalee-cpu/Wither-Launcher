// src-tauri/src/commands/launcher.rs
// Updated with Module 10: Silent Steam Launch

use crate::AppState;
use crate::process::steam_launcher;
use log::info;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResult {
    pub session_id: String,
    pub pid: Option<u32>,
    pub started_at: u64,
}

#[tauri::command]
pub async fn launch_game(
    game_id: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<LaunchResult, String> {
    // Get game info first (release lock before any awaits)
    let game_info = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();

        conn.query_row(
            "SELECT name, source, steam_app_id, executable_path, is_installed 
             FROM games WHERE id = ?",
            params![game_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)? != 0,
            )),
        ).map_err(|e| format!("Game not found: {}", e))?
    };

    let (name, source, steam_app_id, executable_path, is_installed) = game_info;

    info!("Launching game: {} (source: {})", name, source);

    // Emit launch state: starting
    app_handle.emit("game_launch_state", serde_json::json!({
        "status": "starting_steam",
        "game_id": game_id,
    })).ok();

    if source == "steam" {
        // Module 10: Silent Steam Launch
        let app_id = steam_app_id.ok_or("Missing Steam App ID")?;
        
        // Check if Steam is running
        let steam_was_running = steam_launcher::steam_is_running();
        
        // Start Steam if needed
        if !steam_was_running {
            info!("Steam not running, starting silently...");
            steam_launcher::launch_steam_silent().await?;
            
            app_handle.emit("game_launch_state", serde_json::json!({
                "status": "waiting_for_steam",
                "game_id": game_id,
            })).ok();
            
            // Wait for Steam to be ready
            steam_launcher::wait_for_steam_ready(std::time::Duration::from_secs(20)).await?;
        }

        // Launch game via steam:// protocol
        app_handle.emit("game_launch_state", serde_json::json!({
            "status": "launching_game",
            "game_id": game_id,
        })).ok();

        info!("Launching game via steam://rungameid/{}", app_id);
        open::that(format!("steam://rungameid/{}", app_id))
            .map_err(|e| format!("Failed to launch game: {}", e))?;

        // Wait for game process
        let exe_hints = steam_launcher::get_exe_hints(&name, None);
        let game_pid = steam_launcher::wait_for_game_process(
            &exe_hints,
            std::time::Duration::from_secs(30),
        ).await?;

        // Create session
        let session_id = uuid::Uuid::new_v4().to_string();
        let started_at = steam_launcher::unix_now();
        
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let conn = db.get_connection();
            conn.execute(
                "INSERT INTO sessions (id, game_id, started_at) VALUES (?, ?, ?)",
                params![session_id, game_id, started_at],
            ).map_err(|e| format!("Failed to create session: {}", e))?;
        }

        // Emit running state
        app_handle.emit("game_launch_state", serde_json::json!({
            "status": "running",
            "game_id": game_id,
            "session_id": session_id,
            "pid": game_pid,
            "started_at": started_at,
        })).ok();

        // Spawn session watcher (non-blocking)
        let session_clone = session_id.clone();
        let game_clone = game_id.clone();
        let app_handle_clone = app_handle.clone();
        
        info!("=== SPAWNING WATCHER for game {} session {} ===", game_clone, session_clone);
        tokio::spawn(async move {
            steam_launcher::watch_and_save_session(
                game_pid,
                session_clone,
                steam_was_running,
                started_at,
                app_handle_clone,
            ).await;
            
            // Emit exited state
            let _ = app_handle.emit("game_launch_state", serde_json::json!({
                "status": "exited",
                "game_id": game_clone,
            }));
        });
        info!("=== WATCHER SPAWNED ===");

        Ok(LaunchResult {
            session_id,
            pid: Some(game_pid),
            started_at,
        })
    } else {
        // Legacy: direct executable launch
        let exe_path = executable_path.ok_or("Missing executable path")?;

        if !is_installed {
            return Err("Game is not installed".to_string());
        }

        use std::path::Path;
        use std::process::Command;

        let exe_path_obj = Path::new(&exe_path);
        let working_dir = exe_path_obj.parent()
            .ok_or("Invalid executable path")?
            .to_str()
            .ok_or("Invalid working directory")?;

        let child = Command::new(&exe_path)
            .current_dir(working_dir)
            .spawn()
            .map_err(|e| e.to_string())?;

        let pid = child.id();
        let session_id = uuid::Uuid::new_v4().to_string();
        let started_at = steam_launcher::unix_now();

        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let conn = db.get_connection();
            conn.execute(
                "INSERT INTO sessions (id, game_id, started_at) VALUES (?, ?, ?)",
                params![session_id, game_id, started_at],
            ).map_err(|e| format!("Failed to create session: {}", e))?;
        }

        // Emit running state
        app_handle.emit("game_launch_state", serde_json::json!({
            "status": "running",
            "game_id": game_id,
            "session_id": session_id,
            "pid": pid,
            "started_at": started_at,
        })).ok();

        // Spawn session watcher (non-blocking)
        let session_clone = session_id.clone();
        let game_clone = game_id.clone();
        let app_handle_clone = app_handle.clone();
        
        tokio::spawn(async move {
            steam_launcher::watch_and_save_session(
                pid,
                session_clone,
                true, // Don't shutdown Steam for non-Steam games
                started_at,
                app_handle_clone,
            ).await;
            
            // Emit exited state
            let _ = app_handle.emit("game_launch_state", serde_json::json!({
                "status": "exited",
                "game_id": game_clone,
            }));
        });

        Ok(LaunchResult {
            session_id,
            pid: Some(pid),
            started_at,
        })
    }
}

#[tauri::command]
pub fn kill_game(_state: State<'_, AppState>, _game_id: String) -> Result<(), String> {
    // Placeholder - session tracking handled by frontend launchStore
    Ok(())
}
