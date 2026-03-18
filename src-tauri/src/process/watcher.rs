use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use sysinfo::{Pid, ProcessRefreshKind, System};

const POLL_INTERVAL_SECS: u64 = 5;

pub fn watch_process(
    pid: u32,
    session_id: String,
    db: Arc<Mutex<crate::db::Database>>,
) {
    let mut system = System::new();
    let start = SystemTime::now();

    loop {
        std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        
        system.refresh_process_specifics(
            Pid::from(pid as usize),
            ProcessRefreshKind::new(),
        );

        if system.process(Pid::from(pid as usize)).is_none() {
            // Process ended - finalize session
            let duration = start.elapsed().unwrap_or_default().as_secs();
            finalize_session(&db, &session_id, duration as i64);
            break;
        }
    }
}

fn finalize_session(
    db: &Arc<Mutex<crate::db::Database>>,
    session_id: &str,
    duration: i64,
) {
    if let Ok(db_guard) = db.lock() {
        let now = chrono::Utc::now().timestamp();
        let _ = db_guard.conn.execute(
            "UPDATE sessions SET ended_at = ?, duration_s = ? WHERE id = ?",
            rusqlite::params![now, duration, session_id],
        );
    }
}
