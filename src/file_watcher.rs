use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Event>,
    last_change: Instant,
}

impl FileWatcher {
    pub fn new(file_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();

        let mut watcher =
            notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
                if let Ok(event) = event {
                    if event.kind.is_modify() {
                        let _ = tx.send(event);
                    }
                }
            })?;

        watcher.watch(file_path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            last_change: Instant::now(),
        })
    }

    /// Check if the file has changed, with stability checking to avoid multiple events
    /// Returns true if file changed and enough time has passed since last change
    pub fn check_for_changes(&mut self) -> bool {
        // Check for file changes (non-blocking)
        if self.receiver.try_recv().is_ok() {
            // AIDEV-NOTE: Stability check - wait 100ms after file change to avoid multiple events
            let now = Instant::now();
            if now.duration_since(self.last_change) > Duration::from_millis(100) {
                self.last_change = now;
                return true;
            }
        }
        false
    }
}
