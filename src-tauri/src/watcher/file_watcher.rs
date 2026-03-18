use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn start_file_watcher(steamapps_path: &Path, callback: impl Fn() + Send + 'static) {
    let (_tx, rx): (std::sync::mpsc::Sender<Result<Event, notify::Error>>, _) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Check if any .acf file was modified
                for path in event.paths {
                    if path.extension().map_or(false, |ext| ext == "acf") {
                        callback();
                        break;
                    }
                }
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(5)),
    )
    .expect("Failed to create file watcher");

    // Watch the steamapps directory for .acf files
    let _ = watcher.watch(steamapps_path, RecursiveMode::NonRecursive);

    // Keep watcher alive
    loop {
        if rx.recv_timeout(Duration::from_secs(1)).is_err() {
            break;
        }
    }
}
