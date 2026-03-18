use crate::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub duration_s: Option<i64>,
    pub was_crashed: bool,
}

fn row_to_session(row: &rusqlite::Row) -> Result<Session, rusqlite::Error> {
    Ok(Session {
        id: row.get(0)?,
        game_id: row.get(1)?,
        name: String::new(), // Will be filled by get_active_sessions
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        duration_s: row.get(4)?,
        was_crashed: row.get::<_, i64>(5)? != 0,
    })
}

#[tauri::command]
pub fn get_sessions(
    state: State<'_, AppState>,
    game_id: Option<String>,
) -> Result<Vec<Session>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let query = if game_id.is_some() {
        "SELECT id, game_id, started_at, ended_at, duration_s, was_crashed 
         FROM sessions WHERE game_id = ? ORDER BY started_at DESC"
    } else {
        "SELECT id, game_id, started_at, ended_at, duration_s, was_crashed 
         FROM sessions ORDER BY started_at DESC LIMIT 50"
    };

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;

    let rows = if let Some(gid) = game_id {
        stmt.query_map(params![gid], row_to_session)
    } else {
        stmt.query_map([], row_to_session)
    };

    let rows = rows.map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }

    Ok(result)
}

#[tauri::command]
pub fn get_playtime(state: State<'_, AppState>, game_id: String) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let playtime: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(duration_s), 0) FROM sessions WHERE game_id = ?",
            params![game_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(playtime)
}

#[tauri::command]
pub fn get_active_sessions(state: State<'_, AppState>) -> Result<Vec<Session>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.game_id, g.name, s.started_at, s.ended_at, s.duration_s 
             FROM sessions s 
             JOIN games g ON s.game_id = g.id 
             WHERE s.ended_at IS NULL 
             ORDER BY s.started_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let sessions = stmt
        .query_map(params![], |row| {
            Ok(Session {
                id: row.get(0)?,
                game_id: row.get(1)?,
                name: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                duration_s: row.get(5)?,
                was_crashed: false,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for session in sessions {
        result.push(session.map_err(|e| e.to_string())?);
    }

    Ok(result)
}
