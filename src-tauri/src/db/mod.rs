use rusqlite::{Connection, Result};
use std::path::PathBuf;

const SCHEMA: &str = include_str!("schema.sql");

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;
        
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(format!("Could not create directory: {}", e)),
                )
            })?;
        }

        let conn = Connection::open(&db_path)?;
        
        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        // Run schema
        conn.execute_batch(SCHEMA)?;
        
        Ok(Self { conn })
    }

    fn get_db_path() -> Result<PathBuf> {
        let dirs = dirs_next::data_dir()
            .ok_or_else(|| rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Could not determine data directory".to_string())
            ))?;
        
        let app_dir = dirs.join("wither");
        Ok(app_dir.join("wither.db"))
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    pub fn get_connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
