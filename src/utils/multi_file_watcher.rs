use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};

pub struct MultiFileWatcher {
    main_file: PathBuf,
    watchers: HashMap<PathBuf, notify::RecommendedWatcher>,
    receiver: mpsc::Receiver<PathBuf>,
    sender: mpsc::Sender<PathBuf>,
    watched_files: HashSet<PathBuf>,
    last_change: Instant,
}

impl MultiFileWatcher {
    pub fn new(main_file: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = Self {
            main_file: main_file.to_path_buf(),
            watchers: HashMap::new(),
            receiver: rx,
            sender: tx,
            watched_files: HashSet::new(),
            last_change: Instant::now(),
        };

        // Initially watch just the main file
        watcher.add_file_to_watch(main_file)?;
        Ok(watcher)
    }

    fn add_file_to_watch(&mut self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let canonical_path = file_path.canonicalize()?;

        if self.watched_files.contains(&canonical_path) {
            return Ok(()); // Already watching this file
        }

        let tx = self.sender.clone();
        let watch_path = canonical_path.clone();

        let mut watcher =
            notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
                if let Ok(event) = event {
                    if event.kind.is_modify() {
                        let _ = tx.send(watch_path.clone());
                    }
                }
            })?;

        watcher.watch(&canonical_path, RecursiveMode::NonRecursive)?;

        self.watchers.insert(canonical_path.clone(), watcher);
        self.watched_files.insert(canonical_path);

        Ok(())
    }

    fn remove_file_from_watch(&mut self, file_path: &Path) {
        if let Ok(canonical_path) = file_path.canonicalize() {
            self.watchers.remove(&canonical_path);
            self.watched_files.remove(&canonical_path);
        }
    }

    pub fn update_watched_files(
        &mut self,
        all_files: &HashSet<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Always include the main file
        let mut target_files = all_files.clone();
        target_files.insert(self.main_file.clone());

        // Remove files that are no longer needed
        let to_remove: Vec<PathBuf> = self
            .watched_files
            .difference(&target_files)
            .cloned()
            .collect();

        for file_path in to_remove {
            self.remove_file_from_watch(&file_path);
        }

        // Add new files that need watching
        for file_path in &target_files {
            if !self.watched_files.contains(file_path) {
                if let Err(e) = self.add_file_to_watch(file_path) {
                    eprintln!(
                        "Warning: Could not watch file {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Check if any watched file has changed, with stability checking
    /// Returns Some(changed_file_path) if a file changed, None otherwise
    pub fn check_for_changes(&mut self) -> Option<PathBuf> {
        // Check for file changes (non-blocking)
        if let Ok(changed_file) = self.receiver.try_recv() {
            // AIDEV-NOTE: Stability check - wait 100ms after file change to avoid multiple events
            let now = Instant::now();
            if now.duration_since(self.last_change) > Duration::from_millis(100) {
                self.last_change = now;
                return Some(changed_file);
            }
        }
        None
    }
}
