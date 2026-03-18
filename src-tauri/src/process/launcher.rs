use std::path::PathBuf;
use std::process::Command;

pub enum LaunchMethod {
    DirectExecutable { path: PathBuf, args: Vec<String> },
    SteamProtocol { app_id: u32 },
}

pub struct LaunchResult {
    pub pid: Option<u32>,
    pub session_id: String,
    pub started_at: u64,
}

pub enum LaunchError {
    ProcessNotFound,
    PermissionDenied,
    Other(String),
}

pub fn launch_game(method: LaunchMethod) -> Result<LaunchResult, LaunchError> {
    match method {
        LaunchMethod::DirectExecutable { path, args } => {
            let child = Command::new(&path)
                .args(&args)
                .spawn()
                .map_err(|e| LaunchError::Other(e.to_string()))?;

            Ok(LaunchResult {
                pid: Some(child.id()),
                session_id: uuid::Uuid::new_v4().to_string(),
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            })
        }
        LaunchMethod::SteamProtocol { app_id } => {
            let steam_url = format!("steam://rungameid/{}", app_id);
            open::that(&steam_url).ok();

            Ok(LaunchResult {
                pid: None,
                session_id: uuid::Uuid::new_v4().to_string(),
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            })
        }
    }
}
