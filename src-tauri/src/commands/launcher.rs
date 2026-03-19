// src-tauri/src/commands/launcher.rs
// Updated with Module 10: Silent Steam Launch + Linux Proton/native support

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
        // Custom game launch (with Linux native + Proton support)
        let exe_path = executable_path.ok_or("Missing executable path")?;

        if !is_installed {
            return Err("Game is not installed".to_string());
        }

        let child = launch_custom_game(&exe_path, &state)?;

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

/// Launch a custom (non-Steam) game, with Linux-specific handling
fn launch_custom_game(
    exe_path: &str,
    state: &State<'_, AppState>,
) -> Result<std::process::Child, String> {
    use std::path::Path;
    use std::process::Command;

    let exe_path_obj = Path::new(exe_path);
    let working_dir = exe_path_obj.parent()
        .ok_or("Invalid executable path")?;

    #[cfg(target_os = "linux")]
    {
        let extension = exe_path_obj.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Check if it's a Windows .exe → launch via Proton
        if extension == "exe" {
            return launch_via_proton(exe_path, working_dir, state);
        }

        // For .AppImage files, ensure they're executable
        if extension == "appimage" {
            let _ = Command::new("chmod")
                .arg("+x")
                .arg(exe_path)
                .output();
        }

        // For .sh files or extensionless executables, ensure they're executable
        if extension == "sh" || extension.is_empty() {
            let _ = Command::new("chmod")
                .arg("+x")
                .arg(exe_path)
                .output();
        }

        // Launch native Linux executable
        info!("Launching native Linux game: {}", exe_path);
        Command::new(exe_path)
            .current_dir(working_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = state; // suppress unused warning
        Command::new(exe_path)
            .current_dir(working_dir)
            .spawn()
            .map_err(|e| e.to_string())
    }
}

/// Launch a Windows .exe game via Proton on Linux
#[cfg(target_os = "linux")]
fn launch_via_proton(
    exe_path: &str,
    working_dir: &std::path::Path,
    state: &State<'_, AppState>,
) -> Result<std::process::Child, String> {
    use std::process::Command;

    // Get preferred Proton version from settings, or use default
    let proton_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();
        let saved_path: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'proton_path'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        if !saved_path.is_empty() {
            std::path::PathBuf::from(saved_path)
        } else {
            // Use default Proton
            match steam_launcher::get_default_proton() {
                Some(v) => std::path::PathBuf::from(&v.path),
                None => return Err("No Proton version found. Install Proton via Steam or download GE-Proton.".to_string()),
            }
        }
    };

    let proton_exe = proton_path.join("proton");
    if !proton_exe.exists() {
        return Err(format!("Proton executable not found at {:?}", proton_exe));
    }

    // Set up compatibility data paths
    let home = std::env::var("HOME").unwrap_or_default();
    let steam_root = format!("{}/.steam/steam", home);
    let compat_data = format!("{}/.steam/steam/steamapps/compatdata/wither_custom", home);

    // Ensure compat data directory exists
    let _ = std::fs::create_dir_all(&compat_data);

    info!("Launching via Proton: {:?} run {}", proton_exe, exe_path);

    Command::new(&proton_exe)
        .arg("run")
        .arg(exe_path)
        .current_dir(working_dir)
        .env("STEAM_COMPAT_DATA_PATH", &compat_data)
        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_root)
        .spawn()
        .map_err(|e| format!("Failed to launch via Proton: {}", e))
}

#[tauri::command]
pub fn kill_game(_state: State<'_, AppState>, _game_id: String) -> Result<(), String> {
    // Placeholder - session tracking handled by frontend launchStore
    Ok(())
}

/// Get available Proton versions (Linux only, returns empty on other platforms)
#[tauri::command]
pub fn get_proton_versions() -> Result<Vec<steam_launcher::ProtonVersion>, String> {
    Ok(steam_launcher::get_proton_versions())
}

/// Get available GE-Proton releases from GitHub
#[tauri::command]
pub async fn get_proton_ge_releases() -> Result<Vec<steam_launcher::ProtonGeRelease>, String> {
    steam_launcher::fetch_proton_ge_releases().await
}

/// Download and install a GE-Proton release
#[tauri::command]
pub async fn download_proton_ge(
    download_url: String,
    tag_name: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    steam_launcher::download_and_install_proton_ge(&download_url, &tag_name, &app_handle).await
}
