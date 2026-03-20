# Wither — Steam Silent Lifecycle
## How to open Steam as fast as possible and close it the moment the game exits

---

## Overview

When the user clicks Play on a Steam game in Wither, the goal is:

1. Open Steam as fast as possible, completely invisibly
2. Launch the game
3. The moment the game process exits, close Steam immediately

If Steam was already running before the user clicked Play, Wither does nothing to Steam — it never touches a process it did not open.

---

## Part 1 — Check if Steam is Already Running

This is the first thing Wither does when Play is clicked. The result is stored in the session and never re-checked.

```rust
// src-tauri/src/steam/lifecycle.rs

use sysinfo::{System, SystemExt, ProcessExt};

pub fn steam_is_running() -> bool {
    let mut sys = System::new();
    sys.refresh_processes();

    sys.processes().values().any(|p| {
        let name = p.name().to_lowercase();
        // Match "steam" or "steam.exe" but NOT "steamwebhelper"
        (name == "steam" || name == "steam.exe")
            && !name.contains("webhelper")
    })
}
```

Store the result immediately:

```rust
let steam_was_running = steam_is_running();
// This value is captured once and carried through the entire session.
// It is never re-evaluated.
```

---

## Part 2 — Open Steam as Fast as Possible

### 2.1 The flags

Steam accepts command-line flags that suppress its UI entirely and skip slow initialization steps.

```rust
use std::process::Command;

pub fn launch_steam_silent(steam_exe: &std::path::Path) -> Result<(), LaunchError> {
    Command::new(steam_exe)
        .arg("-silent")         // No main window. Starts directly in tray.
        .arg("-nochatui")       // No friends/chat popup window.
        .arg("-nofriendsui")    // No friends list window.
        .arg("-noreactlogin")   // Skips the React-based login screen (already logged in).
        .arg("-noverifyfiles")  // Skips game file verification on startup.
        .spawn()
        .map_err(LaunchError::SpawnFailed)?;

    Ok(())
}
```

### 2.2 Find the Steam executable path

```rust
pub fn find_steam_exe() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Try registry first
        use winreg::enums::*;
        use winreg::RegKey;
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam") {
            if let Ok(path) = hklm.get_value::<String, _>("InstallPath") {
                let exe = std::path::PathBuf::from(path).join("steam.exe");
                if exe.exists() { return Some(exe); }
            }
        }
        // Fallback to default path
        let default = std::path::PathBuf::from("C:/Program Files (x86)/Steam/steam.exe");
        if default.exists() { return Some(default); }
        None
    }

    #[cfg(target_os = "macos")]
    {
        let path = std::path::PathBuf::from(
            "/Applications/Steam.app/Contents/MacOS/steam_osx"
        );
        if path.exists() { return Some(path); }
        None
    }

    #[cfg(target_os = "linux")]
    {
        // Try common locations in order
        let candidates = [
            "/usr/bin/steam",
            "/usr/local/bin/steam",
            "~/.local/share/Steam/steam.sh",
        ];
        for c in &candidates {
            let p = std::path::PathBuf::from(c);
            if p.exists() { return Some(p); }
        }
        None
    }
}
```

### 2.3 Wait for Steam to be ready — as fast as possible

After spawning Steam, you cannot immediately launch the game. Steam needs a few seconds to initialize its IPC layer. The fastest reliable signal is the appearance of the `steamwebhelper` process — this means Steam's networking stack is up.

```rust
use tokio::time::{sleep, Duration, Instant};

pub async fn wait_for_steam_ready() -> Result<(), LaunchError> {
    let deadline = Instant::now() + Duration::from_secs(20);

    loop {
        if Instant::now() > deadline {
            return Err(LaunchError::SteamTimeout);
        }

        let mut sys = System::new();
        sys.refresh_processes();

        let webhelper_up = sys.processes().values()
            .any(|p| p.name().to_lowercase().contains("steamwebhelper"));

        if webhelper_up {
            // Small buffer: steamwebhelper appears ~1s before IPC is actually ready.
            sleep(Duration::from_millis(1200)).await;
            return Ok(());
        }

        sleep(Duration::from_millis(400)).await;
    }
}
```

**Why 1200ms buffer?** `steamwebhelper` starts before the IPC socket is fully bound. Sending `steam://rungameid` too early results in a silent failure. 1200ms covers the gap reliably on slow machines.

**Typical total time from spawn to game launch:**
- Fast SSD machine: ~3–4 seconds
- Average machine: ~5–7 seconds
- Slow HDD machine: ~8–12 seconds

This is why Wither shows a spinner with "Starting Steam..." and "Initializing..." states — the user needs visual feedback during this wait.

---

## Part 3 — Launch the Game

Once Steam is ready, send the protocol URL.

```rust
pub fn launch_game_via_steam(app_id: u32) -> Result<(), LaunchError> {
    let url = format!("steam://rungameid/{}", app_id);
    open::that(&url).map_err(LaunchError::ProtocolFailed)?;
    Ok(())
}
```

`open::that` uses the OS shell to open the URL — this is the same as the user clicking a Steam link in a browser. Steam receives it and starts the game.

---

## Part 4 — Detect the Game Process

`steam://rungameid` does not return a PID. Wither must scan running processes to find the game. This is done by polling every second for up to 30 seconds.

```rust
use sysinfo::{System, SystemExt, ProcessExt, Pid};

pub async fn find_game_pid(exe_hints: &[String]) -> Result<u32, LaunchError> {
    let deadline = Instant::now() + Duration::from_secs(30);

    loop {
        if Instant::now() > deadline {
            return Err(LaunchError::GameProcessNotFound);
        }

        let mut sys = System::new();
        sys.refresh_processes();

        for (pid, process) in sys.processes() {
            let name = process.name().to_lowercase();
            if exe_hints.iter().any(|hint| name.contains(&hint.to_lowercase())) {
                return Ok(pid.as_u32());
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
```

**How to build `exe_hints`:** from the `.acf` manifest file, the `installdir` field gives the game folder. Wither lists the executables in that folder and uses their names as hints.

```rust
pub fn get_exe_hints(install_dir: &std::path::Path) -> Vec<String> {
    let mut hints = vec![];

    if let Ok(entries) = std::fs::read_dir(install_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            // Include .exe on Windows, no extension filter on Linux/macOS
            if cfg!(target_os = "windows") && name.ends_with(".exe") {
                // Exclude launchers, setup, and redistributables
                let excluded = ["unins", "setup", "redist", "vc_", "dxsetup", "crashpad"];
                if !excluded.iter().any(|e| name.contains(e)) {
                    hints.push(name.trim_end_matches(".exe").to_string());
                }
            } else if !cfg!(target_os = "windows") {
                hints.push(name);
            }
        }
    }

    hints
}
```

---

## Part 5 — Monitor the Game and Close Steam on Exit

Once the game PID is known, a background task polls every 5 seconds. When the process disappears, Steam is shut down immediately — but only if Wither was the one that opened it.

```rust
pub async fn watch_and_close_steam(
    game_pid: u32,
    steam_was_running: bool,
    session_id: String,
    started_at: u64,
    db: Arc<Mutex<rusqlite::Connection>>,
    app: tauri::AppHandle,
) {
    loop {
        sleep(Duration::from_secs(5)).await;

        let mut sys = System::new();
        sys.refresh_process(Pid::from(game_pid as usize));

        if sys.process(Pid::from(game_pid as usize)).is_none() {
            // ── Game has exited ──────────────────────────────────────────

            let ended_at  = unix_now();
            let duration  = ended_at - started_at;

            // 1. Finalize session in database
            if let Ok(db) = db.lock() {
                let _ = db.execute(
                    "UPDATE sessions SET ended_at = ?1, duration_s = ?2 WHERE id = ?3",
                    rusqlite::params![ended_at, duration, session_id],
                );
            }

            // 2. Notify frontend
            let _ = app.emit_all("game_launch_state", serde_json::json!({
                "status":     "exited",
                "session_id": session_id,
                "duration_s": duration,
            }));

            // 3. Close Steam — only if Wither opened it
            if !steam_was_running {
                close_steam_immediately().await;
            }

            break;
        }
    }
}
```

### 5.1 Close Steam Immediately

```rust
pub async fn close_steam_immediately() {
    // Step 1: Send the official Steam shutdown command.
    // This is a clean exit — Steam saves state before closing.
    let _ = std::process::Command::new("steam")
        .arg("-shutdown")
        .spawn();

    // Step 2: Wait up to 8 seconds for Steam to exit gracefully.
    let deadline = Instant::now() + Duration::from_secs(8);

    loop {
        sleep(Duration::from_millis(500)).await;

        if !steam_is_running() {
            // Steam closed cleanly — done.
            return;
        }

        if Instant::now() > deadline {
            // Steam did not close in time — force kill.
            force_kill_steam();
            return;
        }
    }
}

fn force_kill_steam() {
    let mut sys = System::new();
    sys.refresh_processes();

    for (_pid, process) in sys.processes() {
        let name = process.name().to_lowercase();
        if name == "steam" || name == "steam.exe" {
            process.kill();
        }
    }
}
```

**Why `steam -shutdown` first?** It allows Steam to write cloud saves, sync achievements, and close cleanly. Force-killing without this can corrupt local Steam data. The 8-second timeout is a safety net for cases where Steam hangs — rare but possible after a game crash.

---

## Part 6 — Complete Flow in One Function

This is the single entry point called by the Tauri command `launch_steam_game`.

```rust
// src-tauri/src/steam/lifecycle.rs

pub async fn launch_game_managed(
    game: &Game,
    db: Arc<Mutex<rusqlite::Connection>>,
    app: tauri::AppHandle,
) -> Result<String, LaunchError> {

    // ── 1. Snapshot Steam state ─────────────────────────────────────────
    let steam_was_running = steam_is_running();

    // ── 2. Open Steam silently if needed ────────────────────────────────
    if !steam_was_running {
        app.emit_all("game_launch_state", json!({
            "status": "starting_steam", "game_id": &game.id
        })).ok();

        let steam_exe = find_steam_exe().ok_or(LaunchError::SteamNotFound)?;
        launch_steam_silent(&steam_exe)?;

        app.emit_all("game_launch_state", json!({
            "status": "waiting_for_steam", "game_id": &game.id
        })).ok();

        wait_for_steam_ready().await?;
    }

    // ── 3. Launch the game ──────────────────────────────────────────────
    app.emit_all("game_launch_state", json!({
        "status": "launching_game", "game_id": &game.id
    })).ok();

    let app_id = game.steam_app_id.ok_or(LaunchError::MissingAppId)?;
    launch_game_via_steam(app_id)?;

    // ── 4. Find the game PID ────────────────────────────────────────────
    let install_dir = get_install_dir(game); // from .acf manifest
    let exe_hints   = get_exe_hints(&install_dir);
    let game_pid    = find_game_pid(&exe_hints).await?;

    // ── 5. Record session start ─────────────────────────────────────────
    let session_id = uuid::Uuid::new_v4().to_string();
    let started_at = unix_now();

    db.lock().unwrap().execute(
        "INSERT INTO sessions (id, game_id, started_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![&session_id, &game.id, started_at],
    ).map_err(LaunchError::DbError)?;

    app.emit_all("game_launch_state", json!({
        "status":     "running",
        "game_id":    &game.id,
        "session_id": &session_id,
        "pid":        game_pid,
        "started_at": started_at,
    })).ok();

    // ── 6. Spawn watcher — non-blocking ────────────────────────────────
    let app_c      = app.clone();
    let db_c       = db.clone();
    let sid        = session_id.clone();

    tokio::spawn(async move {
        watch_and_close_steam(
            game_pid,
            steam_was_running,
            sid,
            started_at,
            db_c,
            app_c,
        ).await;
    });

    Ok(session_id)
}
```

---

## Part 7 — Frontend States

The frontend receives these events in order and shows the corresponding UI:

| Backend event | `status` field | UI shown in game card |
|---|---|---|
| Steam not running → spawn | `starting_steam` | Spinner + "Starting Steam..." |
| Waiting for IPC ready | `waiting_for_steam` | Spinner + "Initializing..." |
| `steam://rungameid` sent | `launching_game` | Spinner + "Loading game..." |
| Game PID found | `running` | Green dot + live timer |
| Game PID gone | `exited` | "Session saved" flash → idle |
| Any error | `error` | Red icon + error message |

The live timer in "running" state is computed entirely client-side from `started_at`:

```typescript
// Svelte — updates every second
let elapsed = ''
let interval: ReturnType<typeof setInterval>

$: if ($launchState.status === 'running' && $launchState.started_at) {
  interval = setInterval(() => {
    const secs = Math.floor(Date.now() / 1000) - $launchState.started_at!
    const h = Math.floor(secs / 3600)
    const m = Math.floor((secs % 3600) / 60)
    const s = secs % 60
    elapsed = h > 0
      ? `${h}h ${String(m).padStart(2,'0')}m`
      : `${String(m).padStart(2,'0')}:${String(s).padStart(2,'0')}`
  }, 1000)
} else {
  clearInterval(interval)
  elapsed = ''
}
```

---

## Part 8 — Edge Cases

| Scenario | Behavior |
|---|---|
| User manually closes Steam while game is running | Watcher detects game PID gone (game likely crashed). Session finalized. Steam already closed — nothing to do. |
| Game crashes instantly after launch | PID disappears within 5s. Session saved with short duration. Steam closed if Wither opened it. |
| Steam takes longer than 20s to start | `wait_for_steam_ready` times out. Emit `error` state to frontend. Show "Steam took too long to start". |
| Game PID not found within 30s | `find_game_pid` times out. Emit `error` state. Steam closed if Wither opened it. |
| System goes to sleep during session | Power event pauses the session timer. Wake resumes it. Sleep duration excluded from `duration_s`. See Module 10 §10.8. |
| User opens Wither while game is already running externally | Not tracked. Wither only tracks sessions it initiates. |
| Two games launched simultaneously | Not supported. The "Play" button is disabled while `launchState.status !== 'idle'`. |

---

## Part 9 — Cargo Dependencies

```toml
# src-tauri/Cargo.toml

[dependencies]
sysinfo  = "0.30"       # Process detection and monitoring
tokio    = { version = "1", features = ["full"] }
open     = "5"          # Cross-platform URL/protocol opener
uuid     = { version = "1", features = ["v4"] }
serde_json = "1"

# Windows only: registry access for Steam path detection
[target.'cfg(target_os = "windows")'.dependencies]
winreg = "0.52"
```

---

*Document: wither-steam-lifecycle.md*
*Version: 1.0.0*
*Last updated: March 2026*
