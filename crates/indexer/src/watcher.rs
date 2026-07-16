//! File watcher for incremental indexing.

use crate::error::{IndexerError, IndexerResult};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

/// File change events.
#[derive(Debug, Clone)]
pub enum FileChange {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

impl FileChange {
    #[must_use]
    pub fn path_key(&self) -> String {
        match self {
            FileChange::Created(p) | FileChange::Modified(p) | FileChange::Deleted(p) => {
                p.to_string_lossy().to_string()
            }
            FileChange::Renamed { to, .. } => to.to_string_lossy().to_string(),
        }
    }
}

/// File watcher that emits change events.
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    root: PathBuf,
    exclude_patterns: Vec<String>,
    tx: mpsc::Sender<FileChange>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    debounce: Arc<Mutex<HashMap<String, tokio::time::Instant>>>,
}

impl FileWatcher {
    /// Create a new file watcher.
    #[allow(clippy::unused_async)]
    pub async fn new(
        root: PathBuf,
        exclude_patterns: Vec<String>,
        tx: mpsc::Sender<FileChange>,
    ) -> IndexerResult<Self> {
        Ok(Self {
            watcher: None,
            root,
            exclude_patterns,
            tx,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debounce: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Start watching.
    #[allow(clippy::unused_async)]
    pub async fn start(&mut self) -> IndexerResult<()> {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(IndexerError::WatcherAlreadyRunning);
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
        let exclude_patterns = self.exclude_patterns.clone();
        let root = self.root.clone();
        let tx_clone = self.tx.clone();
        let is_running = self.is_running.clone();
        let debounce = self.debounce.clone();

        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&self.root, RecursiveMode::Recursive)?;
        self.watcher = Some(watcher);
        self.is_running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Spawn event processor
        tokio::spawn(async move {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                if let Some(event) = rx.recv().await {
                    if let Err(e) =
                        process_event(event, &root, &exclude_patterns, &tx_clone, &debounce).await
                    {
                        warn!("Error processing file event: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop watching.
    #[allow(clippy::unused_async)]
    pub async fn stop(&mut self) {
        self.is_running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.unwatch(&self.root);
        }
    }
}

async fn process_event(
    event: Event,
    root: &Path,
    exclude_patterns: &[String],
    tx: &mpsc::Sender<FileChange>,
    debounce: &Arc<Mutex<HashMap<String, tokio::time::Instant>>>,
) -> IndexerResult<()> {
    // Debounce: ignore events that happen too quickly for the same path
    for path in &event.paths {
        let path_str = path.to_string_lossy().to_string();
        let now = tokio::time::Instant::now();

        let mut debounce_guard = debounce.lock().await;
        if let Some(last) = debounce_guard.get(&path_str) {
            if now.duration_since(*last) < tokio::time::Duration::from_millis(100) {
                continue; // Skip debounced event
            }
        }
        debounce_guard.insert(path_str, now);
        drop(debounce_guard);

        // Check exclude patterns
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let path_str = rel_path.to_string_lossy();
        if exclude_patterns.iter().any(|p| glob_match(p, &path_str)) {
            continue;
        }

        match event.kind {
            EventKind::Create(_) if path.is_file() => {
                tx.send(FileChange::Created(path.clone())).await.ok();
            }
            EventKind::Modify(_) if path.is_file() => {
                tx.send(FileChange::Modified(path.clone())).await.ok();
            }
            EventKind::Remove(_) => {
                tx.send(FileChange::Deleted(path.clone())).await.ok();
            }
            _ => {}
        }
    }
    Ok(())
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.replace("**", "*");
    let regex = pattern
        .replace('.', "\\.")
        .replace('*', ".*")
        .replace('?', ".");
    regex::Regex::new(&format!("^{regex}$")).is_ok_and(|re| re.is_match(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("**/target/**", "src/target/debug/file.rs"));
        assert!(glob_match(
            "**/node_modules/**",
            "project/node_modules/pkg/index.js"
        ));
        assert!(!glob_match("**/target/**", "src/main.rs"));
        assert!(glob_match("*.rs", "file.rs"));
        assert!(!glob_match("*.rs", "file.txt"));
    }
}
