// src-tauri/src/process/steam_launcher.rs
// Module 10 — Silent Steam Launch (cross-platform)

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonVersion {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub source: String, // "custom" or "steam"
}

/// Check if Steam is running.
/// Iterates all system processes for reliable matching on Windows
/// where sysinfo's processes_by_name does exact-name matching.
pub fn steam_is_running() -> bool {
    let mut system = System::new();
    system.refresh_processes();

    for (_pid, process) in system.processes() {
        let name = process.name().to_lowercase();

        // Skip helper/service processes
        if name.contains("steamwebhelper") ||
           name.contains("steamservice") ||
           name.contains("steamerrorreporter") ||
           name == "steamcmd" || name == "steamcmd.exe"
        {
            continue;
        }

        // Match main Steam process
        if name == "steam" || name == "steam.exe" || name == "steam.sh" {
            info!("Steam detected as running (process: {})", name);
            return true;
        }
    }

    false
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

    #[cfg(target_os = "linux")]
    {
        // Check common Linux locations
        let candidates = [
            "/usr/bin/steam",
            "/usr/local/bin/steam",
            "/usr/games/steam",
        ];

        for candidate in &candidates {
            let path = PathBuf::from(candidate);
            if path.exists() {
                return Some(path);
            }
        }

        // Try Flatpak Steam
        let flatpak_steam = PathBuf::from("/var/lib/flatpak/exports/bin/com.valvesoftware.Steam");
        if flatpak_steam.exists() {
            return Some(flatpak_steam);
        }

        // Try user Flatpak
        if let Ok(home) = std::env::var("HOME") {
            let user_flatpak = PathBuf::from(format!(
                "{}/.local/share/flatpak/exports/bin/com.valvesoftware.Steam", home
            ));
            if user_flatpak.exists() {
                return Some(user_flatpak);
            }
        }

        // Fallback: use `which` to find steam
        if let Ok(output) = std::process::Command::new("which").arg("steam").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let steam_app = PathBuf::from("/Applications/Steam.app/Contents/MacOS/steam_osx");
        if steam_app.exists() {
            return Some(steam_app);
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
        system.refresh_processes();

        let mut ready = false;
        for (_pid, process) in system.processes() {
            let name = process.name().to_lowercase();
            if name.contains("steamwebhelper") {
                ready = true;
                break;
            }
        }

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

    // From install directory — scan for actual executables
    if let Some(dir) = install_dir {
        let dir_hints = get_exe_hints_from_install_dir(std::path::Path::new(dir));
        hints.extend(dir_hints);
        // Also use the install dir name itself as a hint
        if let Some(dir_name) = std::path::Path::new(dir).file_name() {
            hints.push(dir_name.to_string_lossy().to_lowercase());
        }
    }

    // From game name
    let name_lower = game_name.to_lowercase();
    hints.push(name_lower.clone());
    hints.push(name_lower.replace(" ", ""));
    hints.push(name_lower.replace(" ", "_"));

    // Deduplicate
    hints.sort();
    hints.dedup();

    hints
}

/// Scan a game's install directory for executable files and return their
/// base names (without extension) as process-detection hints.
/// Scans recursively up to a depth of 3 to find binaries in subfolders like `bin/`.
pub fn get_exe_hints_from_install_dir(install_dir: &std::path::Path) -> Vec<String> {
    let mut hints = Vec::new();
    find_exes_recursive(install_dir, 0, &mut hints);
    hints.sort();
    hints.dedup();
    hints
}

fn find_exes_recursive(dir: &std::path::Path, depth: u8, hints: &mut Vec<String>) {
    if depth > 3 { return; }

    let excluded = ["unins", "setup", "redist", "vc_", "dxsetup", "crashpad",
                    "dotnet", "vcredist", "directx", "_commonredist", "lsi_server"];

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                find_exes_recursive(&path, depth + 1, hints);
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_lowercase();

            #[cfg(target_os = "windows")]
            {
                if name.ends_with(".exe") {
                    if !excluded.iter().any(|e| name.contains(e)) {
                        hints.push(name.trim_end_matches(".exe").to_string());
                    }
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                if path.is_file() {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        use std::os::unix::fs::PermissionsExt;
                        if metadata.permissions().mode() & 0o111 != 0 {
                            if !excluded.iter().any(|e| name.contains(e)) && !name.contains(".") {
                                hints.push(name);
                            }
                        }
                    }
                }
            }
        }
    }
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
                shutdown_steam().await;
            }

            info!("=== WATCHER COMPLETE ===");
            break;
        }
    }
}

/// Shut down Steam (cross-platform)
/// Uses multiple methods for maximum reliability.
async fn shutdown_steam() {
    info!("=== SHUTTING DOWN STEAM ===");

    // Step 1: Try BOTH shutdown methods simultaneously for reliability.
    // Method A: steam -shutdown command (clean exit, saves state)
    let steam_exe = find_steam_exe();
    if let Some(exe) = &steam_exe {
        info!("Sending steam -shutdown via {:?}", exe);
        match std::process::Command::new(exe)
            .arg("-shutdown")
            .spawn()
        {
            Ok(_) => info!("steam -shutdown command sent"),
            Err(e) => error!("Failed to send steam -shutdown: {}", e),
        }
    }

    // Method B: steam://exit protocol (backup)
    info!("Also sending steam://exit protocol...");
    let _ = open::that("steam://exit");

    // Step 2: Wait up to 10 seconds for Steam to exit gracefully.
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;

        if !steam_is_running() {
            info!("Steam closed cleanly.");
            return;
        }

        if Instant::now() > deadline {
            // Steam did not close in time — force kill.
            info!("Steam did not close within 10s, force killing...");
            force_kill_steam();
            return;
        }
    }
}

/// Force-kill all Steam processes using OS-level commands.
fn force_kill_steam() {
    info!("Force-killing Steam processes...");

    #[cfg(target_os = "windows")]
    {
        // taskkill is the most reliable way to kill processes on Windows
        let targets = ["steam.exe"];
        for target in &targets {
            info!("Running: taskkill /f /im {}", target);
            match std::process::Command::new("taskkill")
                .args(["/f", "/im", target])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        info!("Successfully killed {}", target);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        info!("taskkill {} result: {}", target, stderr.trim());
                    }
                }
                Err(e) => error!("Failed to run taskkill: {}", e),
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Use pkill on Linux/macOS
        let _ = std::process::Command::new("pkill")
            .args(["-f", "steam"])
            .output();
        info!("Sent pkill -f steam");
    }
}

/// Get all available Proton versions on the system
#[cfg(target_os = "linux")]
pub fn get_proton_versions() -> Vec<ProtonVersion> {
    let mut versions = Vec::new();
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return versions,
    };

    // 1. Scan custom Proton from compatibilitytools.d
    let compat_dir = PathBuf::from(format!("{}/.steam/steam/compatibilitytools.d", home));
    if compat_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&compat_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                // Check if it has a proton executable
                let proton_exe = path.join("proton");
                if !proton_exe.exists() {
                    continue;
                }

                // Try to read display name from compatibilitytool.vdf
                let display_name = read_compat_vdf_name(&path)
                    .unwrap_or_else(|| {
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });

                // Try to read version file
                let version = read_version_file(&path);

                info!("Found custom Proton: {} at {:?}", display_name, path);

                versions.push(ProtonVersion {
                    name: display_name,
                    path: path.to_string_lossy().to_string(),
                    version,
                    source: "custom".to_string(),
                });
            }
        }
    }

    // Also check alternate compat tools location
    let alt_compat_dir = PathBuf::from(format!("{}/.local/share/Steam/compatibilitytools.d", home));
    if alt_compat_dir.exists() && alt_compat_dir != compat_dir {
        if let Ok(entries) = std::fs::read_dir(&alt_compat_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let proton_exe = path.join("proton");
                if !proton_exe.exists() {
                    continue;
                }

                let display_name = read_compat_vdf_name(&path)
                    .unwrap_or_else(|| {
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });

                let version = read_version_file(&path);

                // Avoid duplicates
                if versions.iter().any(|v| v.name == display_name) {
                    continue;
                }

                info!("Found custom Proton (alt path): {} at {:?}", display_name, path);

                versions.push(ProtonVersion {
                    name: display_name,
                    path: path.to_string_lossy().to_string(),
                    version,
                    source: "custom".to_string(),
                });
            }
        }
    }

    // 2. Scan Steam-installed Proton from steamapps/common
    let steam_common_dirs = vec![
        PathBuf::from(format!("{}/.steam/steam/steamapps/common", home)),
        PathBuf::from(format!("{}/.local/share/Steam/steamapps/common", home)),
    ];

    for common_dir in steam_common_dirs {
        if !common_dir.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&common_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let dir_name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Only match directories starting with "Proton"
                if !dir_name.starts_with("Proton") {
                    continue;
                }

                // Must have a proton executable
                let proton_exe = path.join("proton");
                if !proton_exe.exists() {
                    continue;
                }

                // Try to read version file
                let version = read_version_file(&path);

                // Avoid duplicates (from multiple steam library paths)
                if versions.iter().any(|v| v.name == dir_name) {
                    continue;
                }

                info!("Found Steam Proton: {} at {:?}", dir_name, path);

                versions.push(ProtonVersion {
                    name: dir_name,
                    path: path.to_string_lossy().to_string(),
                    version,
                    source: "steam".to_string(),
                });
            }
        }
    }

    // Sort: custom first, then steam; within each, by name descending (newest first)
    versions.sort_by(|a, b| {
        if a.source != b.source {
            // Custom before steam
            a.source.cmp(&b.source)
        } else {
            // Newer versions first (reverse alphabetical typically works for version names)
            b.name.cmp(&a.name)
        }
    });

    versions
}

#[cfg(not(target_os = "linux"))]
pub fn get_proton_versions() -> Vec<ProtonVersion> {
    Vec::new() // Proton is Linux-only
}

/// Read display_name from a compatibilitytool.vdf file
#[cfg(target_os = "linux")]
fn read_compat_vdf_name(proton_dir: &PathBuf) -> Option<String> {
    let vdf_path = proton_dir.join("compatibilitytool.vdf");
    if !vdf_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&vdf_path).ok()?;

    // Parse "display_name" "GE-Proton10-32" from the VDF
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("\"display_name\"") {
            // Extract value between quotes after the key
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 4 {
                return Some(parts[3].to_string());
            }
        }
    }

    None
}

/// Read version from the "version" file in a Proton directory
#[cfg(target_os = "linux")]
fn read_version_file(proton_dir: &PathBuf) -> Option<String> {
    let version_path = proton_dir.join("version");
    if !version_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&version_path).ok()?;
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return None;
    }

    // Version file can be just a string like "GE-Proton10-32"
    // or "1773313326 experimental-10.0-20260312"
    // Return the meaningful part
    if trimmed.contains(' ') {
        // Second part is the version string
        Some(trimmed.split_whitespace().last().unwrap_or(trimmed).to_string())
    } else {
        Some(trimmed.to_string())
    }
}

/// Get the default/best Proton version
#[cfg(target_os = "linux")]
pub fn get_default_proton() -> Option<ProtonVersion> {
    let versions = get_proton_versions();

    // Prefer Proton Experimental
    if let Some(v) = versions.iter().find(|v| v.name.contains("Experimental")) {
        return Some(v.clone());
    }

    // Then GE-Proton (custom)
    if let Some(v) = versions.iter().find(|v| v.name.starts_with("GE-Proton")) {
        return Some(v.clone());
    }

    // Then any Proton Hotfix
    if let Some(v) = versions.iter().find(|v| v.name.contains("Hotfix")) {
        return Some(v.clone());
    }

    // Fallback: first available
    versions.into_iter().next()
}

// ── GE-Proton Download Manager ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonGeRelease {
    pub tag_name: String,
    pub name: String,
    pub download_url: String,
    pub size_bytes: u64,
    pub published_at: String,
    pub is_installed: bool,
}

/// Fetch available GE-Proton releases from GitHub
#[cfg(target_os = "linux")]
pub async fn fetch_proton_ge_releases() -> Result<Vec<ProtonGeRelease>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Wither-Launcher/0.1")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases?per_page=10";
    
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let releases: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse releases: {}", e))?;

    let installed_versions = get_proton_versions();

    let mut result = Vec::new();
    for release in releases {
        let tag_name = release["tag_name"].as_str().unwrap_or("").to_string();
        let name = release["name"].as_str().unwrap_or(&tag_name).to_string();
        let published_at = release["published_at"].as_str().unwrap_or("").to_string();

        // Find the .tar.gz asset
        if let Some(assets) = release["assets"].as_array() {
            for asset in assets {
                let asset_name = asset["name"].as_str().unwrap_or("");
                if asset_name.ends_with(".tar.gz") {
                    let download_url = asset["browser_download_url"].as_str().unwrap_or("").to_string();
                    let size_bytes = asset["size"].as_u64().unwrap_or(0);

                    let is_installed = installed_versions.iter().any(|v| {
                        v.name.contains(&tag_name) || tag_name.contains(&v.name)
                    });

                    result.push(ProtonGeRelease {
                        tag_name: tag_name.clone(),
                        name: name.clone(),
                        download_url,
                        size_bytes,
                        published_at,
                        is_installed,
                    });
                    break;
                }
            }
        }
    }

    Ok(result)
}

#[cfg(not(target_os = "linux"))]
pub async fn fetch_proton_ge_releases() -> Result<Vec<ProtonGeRelease>, String> {
    Ok(Vec::new())
}

/// Download and install a GE-Proton release
#[cfg(target_os = "linux")]
pub async fn download_and_install_proton_ge(
    download_url: &str,
    tag_name: &str,
    app_handle: &tauri::AppHandle,
) -> Result<String, String> {
    use std::io::Write;

    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let compat_dir = PathBuf::from(format!("{}/.steam/steam/compatibilitytools.d", home));

    // Create compat dir if it doesn't exist
    std::fs::create_dir_all(&compat_dir)
        .map_err(|e| format!("Failed to create compatibilitytools.d: {}", e))?;

    // Emit: starting download
    let _ = app_handle.emit("proton_download_progress", serde_json::json!({
        "stage": "downloading",
        "tag": tag_name,
        "percent": 0,
    }));

    info!("Downloading GE-Proton {} from {}", tag_name, download_url);

    let client = reqwest::Client::builder()
        .user_agent("Wither-Launcher/0.1")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    
    // Download to temp file
    let tmp_path = std::env::temp_dir().join(format!("{}.tar.gz", tag_name));
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            // Only emit every ~5% to avoid flooding
            if percent % 5 == 0 {
                let _ = app_handle.emit("proton_download_progress", serde_json::json!({
                    "stage": "downloading",
                    "tag": tag_name,
                    "percent": percent,
                }));
            }
        }
    }

    drop(file);

    info!("Download complete ({}). Extracting...", downloaded);

    // Emit: extracting
    let _ = app_handle.emit("proton_download_progress", serde_json::json!({
        "stage": "extracting",
        "tag": tag_name,
        "percent": 100,
    }));

    // Extract tar.gz to compatibilitytools.d
    let output = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&tmp_path)
        .arg("-C")
        .arg(&compat_dir)
        .output()
        .map_err(|e| format!("Failed to run tar: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Extraction failed: {}", stderr));
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&tmp_path);

    // Emit: done
    let _ = app_handle.emit("proton_download_progress", serde_json::json!({
        "stage": "done",
        "tag": tag_name,
        "percent": 100,
    }));

    info!("GE-Proton {} installed successfully to {:?}", tag_name, compat_dir);

    Ok(format!("GE-Proton {} installed successfully", tag_name))
}

#[cfg(not(target_os = "linux"))]
pub async fn download_and_install_proton_ge(
    _download_url: &str,
    _tag_name: &str,
    _app_handle: &tauri::AppHandle,
) -> Result<String, String> {
    Err("Proton is only available on Linux".to_string())
}
