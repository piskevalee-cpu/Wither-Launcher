#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod process;
mod steam;
mod watcher;
mod tray;

use db::Database;
use log::info;
use std::sync::{Arc, Mutex};
use tauri::{Manager, RunEvent};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Wither Launcher...");

    // Initialize database
    let db = Database::new().expect("Failed to initialize database");
    let db = Arc::new(Mutex::new(db));

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { db })
        .invoke_handler(tauri::generate_handler![
            commands::library::get_all_games,
            commands::library::add_custom_game,
            commands::library::remove_game,
            commands::library::update_custom_game,
            commands::library::update_game_last_played,
            commands::library::read_file_bytes,
            commands::library::get_setting,
            commands::library::set_setting,
            commands::launcher::launch_game,
            commands::launcher::kill_game,
            commands::steam::steam_login,
            commands::steam::steam_logout,
            commands::steam::get_steam_user,
            commands::steam::sync_steam,
            commands::steam::get_steam_games,
            commands::steam::debug_steam_paths,
            commands::steam::reset_steam_games,
            commands::steam::clear_removed_steam_games,
            commands::steam::read_acf_file,
            commands::stats::get_sessions,
            commands::stats::get_playtime,
            commands::stats::get_active_sessions,
            commands::store::store_get_featured,
            commands::store::store_get_categories,
            commands::store::store_get_app,
            commands::store::store_search,
            commands::store::store_browse,
            commands::store::open_url,
        ])
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize system tray
            tray::setup_tray(&app.handle())?;

            // Run startup Steam sync (once, not periodic)
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = handle.state::<AppState>();
                let _ = steam::sync_steam_library(&state);
            });

            info!("Wither Launcher setup complete");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        match event {
            RunEvent::ExitRequested { api, .. } => {
                // Check settings for close behavior
                let close_to_tray = {
                    let state = app_handle.state::<AppState>();
                    let db = state.db.lock().unwrap();
                    let value: String = db.conn
                        .query_row("SELECT value FROM settings WHERE key = ?", ["close_to_tray"], |row| row.get(0))
                        .unwrap_or("true".to_string());
                    value == "true"
                };
                
                if close_to_tray {
                    api.prevent_exit();
                }
            }
            RunEvent::WindowEvent { label, event, .. } => {
                if label == "main" {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            // Check settings for close behavior
                            let close_to_tray = {
                                let state = app_handle.state::<AppState>();
                                let db = state.db.lock().unwrap();
                                let value: String = db.conn
                                    .query_row("SELECT value FROM settings WHERE key = ?", ["close_to_tray"], |row| row.get(0))
                                    .unwrap_or("true".to_string());
                                value == "true"
                            };
                            
                            if close_to_tray {
                                // Hide to tray
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                                api.prevent_close();
                            }
                            // If close_to_tray is false, let the window close
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
