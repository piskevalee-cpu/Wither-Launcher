// src-tauri/src/process/steam_launcher.rs
// Module 10 — Silent Steam Launch

use log::{info, error};
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, System, UpdateKind};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchContext {
    pub steam_was_running: bool,
    pub steam_pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LaunchStatus {
    Idle,
    StartingSteam,
    WaitingForSteam,
    LaunchingGame,
    Running { pid: u32, started_at: u64 },
    Exited { duration_s: u64 },
    Error { message: String },
}

/// Check if Steam is running (excluding steamwebhelper)
pub fn steam_is_running() -> bool {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessRefreshKind::new()
            .with_exe(UpdateKind::OnlyIfNotSet)
    );
    
    let steam_running = system.processes_by_name("steam")
        .any(|p| {
            let name = p.name().to_lowercase();
            !name.contains("steamwebhelper")
        });
    
    steam_running
}

/// Find Steam executable path
pub fn find_steam_exe() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        
        // Try WOW6432Node first (64-bit Windows)
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                let steam_exe = PathBuf::from(path).join("steam.exe");
                if steam_exe.exists() {
                    return Some(steam_exe);
                }
            }
        }
        
        // Try regular key (32-bit Windows)
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Valve\\Steam") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                let steam_exe = PathBuf::from(path).join("steam.exe");
                if steam_exe.exists() {
                    return Some(steam_exe);
                }
            }
        }
        
        // Fallback to default path
        let default = PathBuf::from(r"C:\Program Files (x86)\Steam\steam.exe");
        if default.exists() {
            return Some(default);
        }
    }
    
    None
}

/// Launch Steam silently with minimal UI
pub async fn launch_steam_silent() -> Result<(), String> {
    let steam_exe = find_steam_exe()
        .ok_or("Steam executable not found")?;
    
    info!("Launching Steam silently: {:?}", steam_exe);
    
    let _child = std::process::Command::new(&steam_exe)
        .arg("-silent")        // No main window
        .arg("-nochatui")      // No chat popup
        .arg("-nofriendsui")   // No friends list
        .arg("-noreactlogin")  // Skip React login if already logged in
        .spawn()
        .map_err(|e| format!("Failed to spawn Steam: {}", e))?;
    
    info!("Steam launched successfully");
    Ok(())
}

/// Wait for Steam to be ready (steamwebhelper process appears)
pub async fn wait_for_steam_ready(timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    
    info!("Waiting for Steam to be ready...");
    
    loop {
        if Instant::now() > deadline {
            return Err("Steam failed to start within timeout".to_string());
        }
        
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessRefreshKind::new()
                .with_exe(UpdateKind::OnlyIfNotSet)
        );
        
        let ready = system.processes_by_name("steamwebhelper").count() > 0;
        
        if ready {
            // Extra buffer: Steam needs ~1.5s after webhelper appears
            tokio::time::sleep(Duration::from_millis(1500)).await;
            info!("Steam is ready!");
            return Ok(());
        }
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Get executable hints for game process detection
pub fn get_exe_hints(game_name: &str, install_dir: Option<&str>) -> Vec<String> {
    let mut hints = Vec::new();
    
    // From install directory
    if let Some(dir) = install_dir {
        hints.push(dir.to_lowercase());
    }
    
    // From game name
    let name_lower = game_name.to_lowercase();
    hints.push(name_lower.clone());
    hints.push(name_lower.replace(" ", ""));
    hints.push(name_lower.replace(" ", "_"));
    
    hints
}

/// Wait for game process to appear
pub async fn wait_for_game_process(
    exe_hints: &[String],
    timeout: Duration,
) -> Result<u32, String> {
    let deadline = Instant::now() + timeout;
    
    info!("Waiting for game process (hints: {:?})", exe_hints);
    
    loop {
        if Instant::now() > deadline {
            return Err("Game process not found within timeout".to_string());
        }
        
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessRefreshKind::new()
                .with_exe(UpdateKind::OnlyIfNotSet)
        );
        
        for (pid, process) in system.processes() {
            let name = process.name().to_lowercase();
            if exe_hints.iter().any(|h| name.contains(h)) {
                info!("Found game process: {} (PID: {})", name, pid);
                return Ok(pid.as_u32());
            }
        }
        
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

/// Get Unix timestamp
pub fn unix_now() -> u64 {
    chrono::Utc::now().timestamp() as u64
}

/// Watch game process and save session when it exits
pub async fn watch_and_save_session(
    game_pid: u32,
    session_id: String,
    steam_was_running: bool,
    started_at: u64,
    app_handle: tauri::AppHandle,
) {
    info!("=== WATCHER STARTED ===");
    info!("Watching game process (PID: {})", game_pid);
    info!("Session: {} | Started: {} | Steam was running: {}", session_id, started_at, steam_was_running);

    let mut last_check = Instant::now();
    let mut child_pids: Vec<u32> = Vec::new();

    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let mut system = System::new();
        system.refresh_processes();

        // Check main process
        let main_process_exists = system.process(sysinfo::Pid::from(game_pid as usize)).is_some();
        
        // Also check for child processes (some games spawn children)
        let mut child_process_exists = false;
        for (pid, process) in system.processes() {
            let parent = process.parent().map(|p| p.as_u32());
            if parent == Some(game_pid) || child_pids.contains(&pid.as_u32()) {
                child_process_exists = true;
                if !child_pids.contains(&pid.as_u32()) {
                    child_pids.push(pid.as_u32());
                    info!("Found child process: {} (PID: {})", process.name(), pid);
                }
            }
        }

        let any_process_exists = main_process_exists || child_process_exists;
        
        // Log every 30 seconds
        if last_check.elapsed() >= Duration::from_secs(30) {
            info!("Game still running... (main: {}, children: {})", main_process_exists, child_process_exists);
            last_check = Instant::now();
        }
        
        if !any_process_exists {
            info!("=== GAME EXITED DETECTED ===");
            let ended_at = unix_now();
            let duration_s = ended_at.saturating_sub(started_at);

            info!("Game exited. Duration: {}s", duration_s);

            // Save session to database
            match crate::db::Database::new() {
                Ok(db) => {
                    let conn = db.get_connection();
                    match conn.execute(
                        "UPDATE sessions SET ended_at = ?, duration_s = ? WHERE id = ?",
                        rusqlite::params![ended_at as i64, duration_s as i64, session_id],
                    ) {
                        Ok(rows) => {
                            info!("Session saved: {} ({}s, rows: {})", session_id, duration_s, rows);
                            // Emit event to frontend
                            let _ = app_handle.emit("session_updated", serde_json::json!({
                                "session_id": session_id,
                                "duration_s": duration_s,
                            }));
                        },
                        Err(e) => error!("Failed to save session {}: {}", session_id, e),
                    }
                }
                Err(e) => error!("Failed to open DB for session save: {}", e),
            }

            // Shut down Steam if we started it
            if !steam_was_running {
                info!("=== SHUTTING DOWN STEAM ===");
                // Method 1: steam://exit protocol
                info!("Trying steam://exit protocol...");
                let _ = open::that("steam://exit");
                
                tokio::time::sleep(Duration::from_secs(3)).await;
                
                // Method 2: Kill steam.exe processes
                info!("Killing Steam processes...");
                let mut system = System::new();
                system.refresh_processes();
                let mut killed = 0;
                for (pid, process) in system.processes() {
                    let name = process.name().to_lowercase();
                    if name == "steam.exe" || (name.contains("steam") && !name.contains("steamwebhelper")) {
                        info!("Killing Steam process: {} (PID: {})", name, pid);
                        process.kill();
                        killed += 1;
                    }
                }
                info!("Killed {} Steam process(es)", killed);
            }
            
            info!("=== WATCHER COMPLETE ===");
            break;
        }
    }
}
