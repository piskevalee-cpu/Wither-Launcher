// src-tauri/src/steam/acf_parser.rs
// Steam .acf Manifest File Parser with Multiple Library Support

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AcfGame {
    pub app_id: u32,
    pub name: String,
    pub install_dir: String,
    pub state_flags: u32,
    pub last_updated: u64,
    pub size_on_disk: u64,
    pub library_path: PathBuf,
}

/// Parse a single .acf file and extract game info
pub fn parse_acf_file(path: &Path) -> Result<AcfGame, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read ACF file: {}", e))?;

    let app_id = extract_value(&content, "appid")
        .ok_or("Missing appid")?
        .parse::<u32>()
        .map_err(|e| format!("Invalid appid: {}", e))?;

    let name = extract_value(&content, "name")
        .ok_or("Missing name")?
        .to_string();

    let install_dir = extract_value(&content, "installdir")
        .ok_or("Missing installdir")?
        .to_string();

    let state_flags = extract_value(&content, "StateFlags")
        .unwrap_or("0")
        .parse::<u32>()
        .unwrap_or(0);

    log::info!("Parsed ACF: {} (AppID: {}, StateFlags: {})", name, app_id, state_flags);

    let last_updated = extract_value(&content, "LastUpdated")
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    let size_on_disk = extract_value(&content, "SizeOnDisk")
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    let library_path = path.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(AcfGame {
        app_id,
        name,
        install_dir,
        state_flags,
        last_updated,
        size_on_disk,
        library_path,
    })
}

/// Extract a value from .acf file content
fn extract_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    for line in content.lines() {
        let line = line.trim();
        // More flexible parsing - handle various formats
        if line.contains(&format!("\"{}\"", key)) {
            // Find the value after the key
            if let Some(key_end) = line.find(&format!("\"{}\"", key)).map(|pos| pos + key.len() + 2) {
                let rest = &line[key_end..];
                // Find the quoted value
                if let Some(value_start) = rest.find('"') {
                    let after_quote = &rest[value_start + 1..];
                    if let Some(value_end) = after_quote.find('"') {
                        return Some(&after_quote[..value_end]);
                    }
                }
            }
        }
    }
    None
}

/// Parse libraryfolders.vdf to get all Steam library paths
pub fn parse_library_folders(vdf_path: &Path) -> Result<Vec<PathBuf>, String> {
    let content = fs::read_to_string(vdf_path)
        .map_err(|e| format!("Failed to read libraryfolders.vdf: {}", e))?;

    let mut paths = Vec::new();
    
    // Parse paths from the VDF file
    // Format can be:
    // "0" "C:\\Program Files (x86)\\Steam"
    // or newer format with "path" key
    for line in content.lines() {
        let line = line.trim();
        
        // Try to extract path from quoted strings
        if line.contains('"') {
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 3 {
                let potential_path = parts[1].trim();
                
                // Check if it looks like a valid path
                if potential_path.contains(':') || potential_path.starts_with('/') || potential_path.starts_with('\\') {
                    let path = PathBuf::from(potential_path.replace("\\\\", "\\"));
                    if path.exists() {
                        paths.push(path);
                    }
                }
            }
        }
    }

    // Always include the default Steam path if it exists
    #[cfg(target_os = "windows")]
    {
        let default_path = PathBuf::from("C:\\Program Files (x86)\\Steam");
        if default_path.exists() && !paths.contains(&default_path) {
            paths.push(default_path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let default_path = PathBuf::from(format!("{}/.steam/steam", home));
            if default_path.exists() && !paths.contains(&default_path) {
                paths.push(default_path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let default_path = PathBuf::from(format!("{}/Library/Application Support/Steam", home));
            if default_path.exists() && !paths.contains(&default_path) {
                paths.push(default_path);
            }
        }
    }

    Ok(paths)
}

/// Scan all library folders for installed games
pub fn scan_all_libraries(steam_roots: &[PathBuf]) -> Vec<AcfGame> {
    let mut all_games = Vec::new();

    for steam_root in steam_roots {
        let steamapps_path = steam_root.join("steamapps");
        if !steamapps_path.exists() {
            continue;
        }

        // Scan for .acf files
        if let Ok(entries) = fs::read_dir(&steamapps_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "acf") 
                    && path.file_name().map_or(false, |name| name.to_string_lossy().starts_with("appmanifest_"))
                {
                    if let Ok(game) = parse_acf_file(&path) {
                        // Only include fully installed games (StateFlags = 4)
                        if game.state_flags == 4 || game.state_flags == 6 {
                            all_games.push(game);
                        }
                    }
                }
            }
        }
    }

    all_games
}

/// Get the executable path for a game
pub fn get_game_executable_path(game: &AcfGame) -> PathBuf {
    let install_path = game.library_path.join("common").join(&game.install_dir);
    
    // Try common executable names
    let exe_names = [
        format!("{}.exe", game.name.to_lowercase().replace(" ", "")),
        format!("{}.exe", game.install_dir.to_lowercase().replace(" ", "")),
        "game.exe".to_string(),
        "main.exe".to_string(),
    ];

    for exe_name in &exe_names {
        let potential_path = install_path.join(exe_name);
        if potential_path.exists() {
            return potential_path;
        }
    }

    // Fallback: find the largest .exe in the directory
    if let Ok(entries) = fs::read_dir(&install_path) {
        let mut largest_exe: Option<(PathBuf, u64)> = None;
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "exe") {
                if let Ok(metadata) = fs::metadata(&path) {
                    let size = metadata.len();
                    if largest_exe.as_ref().map_or(true, |(_, largest_size)| size > *largest_size) {
                        largest_exe = Some((path, size));
                    }
                }
            }
        }

        if let Some((path, _)) = largest_exe {
            return path;
        }
    }

    // Last resort: return install directory
    install_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_value() {
        let content = r#"
"AppState"
{
    "appid"     "570"
    "name"      "Dota 2"
    "installdir" "dota 2 beta"
}"#;
        
        assert_eq!(extract_value(content, "appid"), Some("570"));
        assert_eq!(extract_value(content, "name"), Some("Dota 2"));
        assert_eq!(extract_value(content, "installdir"), Some("dota 2 beta"));
        assert_eq!(extract_value(content, "nonexistent"), None);
    }
}
