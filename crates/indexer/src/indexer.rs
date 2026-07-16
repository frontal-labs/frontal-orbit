//! Incremental indexer that coordinates file watching, parsing, and graph updates.

use crate::config::IndexerConfig;
use crate::error::{IndexerError, IndexerResult};
use crate::languages::create_indexer;
use crate::watcher::{FileChange, FileWatcher};
use crate::{
    CallEdge, CodeGraph, CoverageEdge, CoverageType, FileId, ImportEdge, IndexedFile, Language,
    Symbol, SymbolId, SymbolKind,
};
use orbit_memory::{MemoryScope, MemoryService};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Incremental indexer that maintains a code graph.
pub struct IncrementalIndexer {
    config: IndexerConfig,
    graph: Arc<RwLock<CodeGraph>>,
    file_hashes: Arc<RwLock<HashMap<PathBuf, String>>>,
    memory: Option<Arc<MemoryService>>,
    scope: MemoryScope,
    watcher: Option<FileWatcher>,
    is_running: Arc<RwLock<bool>>,
}

impl IncrementalIndexer {
    /// Create a new incremental indexer.
    #[allow(clippy::unused_async)]
    pub async fn new(config: IndexerConfig) -> IndexerResult<Self> {
        let memory = if config.generate_embeddings {
            Some(Arc::new(MemoryService::from_env()))
        } else {
            None
        };

        let scope = MemoryScope::new(config.workspace_root.to_string_lossy().to_string());

        Ok(Self {
            config,
            graph: Arc::new(RwLock::new(CodeGraph::new())),
            file_hashes: Arc::new(RwLock::new(HashMap::new())),
            memory,
            scope,
            watcher: None,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Get the code graph.
    #[must_use]
    pub fn graph(&self) -> Arc<RwLock<CodeGraph>> {
        self.graph.clone()
    }

    /// Perform a full re-index of the workspace.
    pub async fn full_reindex(&self) -> IndexerResult<CodeGraph> {
        info!(
            "Starting full re-index of {}",
            self.config.workspace_root.display()
        );

        let mut graph = self.graph.write().await;
        graph.clear();

        let mut file_hashes = self.file_hashes.write().await;
        file_hashes.clear();

        let files = self.discover_files().await?;
        info!("Found {} files to index", files.len());

        for file_path in files {
            if let Err(e) = self
                .index_single_file(&mut graph, &mut file_hashes, &file_path)
                .await
            {
                warn!("Failed to index {}: {}", file_path.display(), e);
            }
        }

        // Build derived indexes
        graph.build_indexes();

        // Sync to memory backend if available
        if let Some(memory) = &self.memory {
            if let Err(e) = self.sync_to_memory(&graph, memory).await {
                warn!("Failed to sync to memory backend: {}", e);
            }
        }

        info!(
            "Re-index complete: {} files, {} symbols",
            graph.files.len(),
            graph.symbols.len()
        );
        Ok((*graph).clone())
    }

    /// Discover all files to index in the workspace.
    #[allow(clippy::unused_async)]
    async fn discover_files(&self) -> IndexerResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let root = &self.config.workspace_root;

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();
            if path.is_file() {
                // Check exclude patterns
                if self.should_exclude(path) {
                    continue;
                }

                // Check file size
                if let Ok(metadata) = path.metadata() {
                    if metadata.len() > self.config.max_file_size as u64 {
                        debug!(
                            "Skipping large file: {} ({} bytes)",
                            path.display(),
                            metadata.len()
                        );
                        continue;
                    }
                }

                // Check if we have an indexer for this language
                let lang = Language::from_path(path);
                if lang != Language::Unknown && lang.supports_ast() {
                    files.push(path.to_path_buf());
                }
            }
        }

        Ok(files)
    }

    fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.config.exclude_patterns {
            if glob_match(pattern, &path_str) {
                return true;
            }
        }
        false
    }

    /// Index a single file and update the graph.
    async fn index_single_file(
        &self,
        graph: &mut CodeGraph,
        file_hashes: &mut HashMap<PathBuf, String>,
        file_path: &Path,
    ) -> IndexerResult<()> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let hash = compute_hash(&content);

        // Check if file has changed
        if file_hashes.get(file_path) == Some(&hash) {
            return Ok(()); // Unchanged
        }

        let lang = Language::from_path(file_path);
        let indexer = create_indexer(lang);

        let indexed_file = indexer.index_file(file_path, &content)?;

        // Remove old symbols/edges for this file if it existed
        if graph.files.contains_key(&indexed_file.id) {
            self.remove_file_data(graph, indexed_file.id);
        }

        // Add to graph
        let _file_id = graph.add_file(indexed_file.clone());
        file_hashes.insert(file_path.to_path_buf(), hash);

        // Add call edges, imports, etc.
        self.add_file_edges(graph, &indexed_file).await?;

        debug!("Indexed: {}", file_path.display());
        Ok(())
    }

    /// Remove old data for a file from the graph.
    #[allow(clippy::unused_self)]
    fn remove_file_data(&self, graph: &mut CodeGraph, file_id: FileId) {
        Self::remove_file_data_static(graph, file_id);
    }

    /// Add edges for a newly indexed file.
    #[allow(clippy::unused_async)]
    async fn add_file_edges(&self, graph: &mut CodeGraph, file: &IndexedFile) -> IndexerResult<()> {
        // Add call edges from references
        for ref_ in &file.references {
            if let Some(callee) = graph.symbols.get(&ref_.symbol_id) {
                if let Some(caller) =
                    Self::find_enclosing_function(graph, file.id, ref_.span.start_line)
                {
                    let edge = CallEdge {
                        caller: caller.id.clone(),
                        callee: callee.id.clone(),
                        call_site: ref_.span,
                        is_dynamic: false,
                        is_async: callee.is_async,
                    };
                    graph.add_call(edge);
                }
            }
        }

        // Add import edges
        for import in &file.imports {
            if let Some(to_file) = graph.get_file_by_path(&PathBuf::from(&import.path)) {
                let edge = ImportEdge {
                    from_file: file.id,
                    to_file: to_file.id,
                    import: import.clone(),
                };
                graph.add_import(edge);
            }
        }

        // Add coverage edges (test functions -> covered functions)
        if self.config.index_tests {
            for symbol in &file.symbols {
                if self.is_test_function(symbol) {
                    // Find functions in the same module that this test might cover
                    for other in &file.symbols {
                        if matches!(
                            other.kind,
                            SymbolKind::Function | SymbolKind::AsyncFunction | SymbolKind::Method
                        ) && other.name != symbol.name
                            && !self.is_test_function(other)
                        {
                            let edge = CoverageEdge {
                                test_symbol: symbol.id.clone(),
                                covered_symbol: other.id.clone(),
                                coverage_type: CoverageType::Unit,
                            };
                            graph.add_coverage(edge);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the function enclosing a given line.
    fn find_enclosing_function(graph: &CodeGraph, file_id: FileId, line: u32) -> Option<&Symbol> {
        graph.files.get(&file_id)?.symbols.iter().find(|s| {
            s.span.start_line <= line
                && s.span.end_line >= line
                && matches!(
                    s.kind,
                    SymbolKind::Function | SymbolKind::AsyncFunction | SymbolKind::Method
                )
        })
    }

    #[allow(clippy::unused_self)]
    fn is_test_function(&self, symbol: &Symbol) -> bool {
        symbol.name.starts_with("test_")
            || symbol.attributes.iter().any(|a| a.contains("test"))
            || symbol.attributes.iter().any(|a| a.contains("bench"))
    }

    /// Build derived indexes after full reindex.
    pub fn build_indexes(&self) {
        // The graph builds its indexes incrementally in add_file/add_symbol/etc.
    }

    /// Sync graph to persistent memory backend.
    async fn sync_to_memory(
        &self,
        graph: &CodeGraph,
        memory: &Arc<MemoryService>,
    ) -> IndexerResult<()> {
        use crate::graph::GraphSyncer;
        let syncer = GraphSyncer::new(memory.clone(), self.scope.clone());
        syncer.sync_graph(graph).await
    }

    /// Start the file watcher for incremental updates.
    pub async fn start_watching(&mut self) -> IndexerResult<()> {
        if *self.is_running.read().await {
            return Err(IndexerError::WatcherAlreadyRunning);
        }

        let (tx, rx) = mpsc::channel(1000);
        let mut watcher = FileWatcher::new(
            self.config.workspace_root.clone(),
            self.config.exclude_patterns.clone(),
            tx,
        )
        .await?;

        watcher.start().await?;
        self.watcher = Some(watcher);
        *self.is_running.write().await = true;

        // Start processing loop
        let graph = self.graph.clone();
        let file_hashes = self.file_hashes.clone();
        let config = self.config.clone();
        let memory = self.memory.clone();
        let scope = self.scope.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            Self::process_changes(graph, file_hashes, config, memory, scope, rx, is_running).await;
        });

        info!("File watcher started");
        Ok(())
    }

    /// Stop the file watcher.
    pub async fn stop_watching(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            watcher.stop().await;
        }
        *self.is_running.write().await = false;
        info!("File watcher stopped");
    }

    /// Process file change events.
    async fn process_changes(
        graph: Arc<RwLock<CodeGraph>>,
        file_hashes: Arc<RwLock<HashMap<PathBuf, String>>>,
        config: IndexerConfig,
        memory: Option<Arc<MemoryService>>,
        scope: MemoryScope,
        mut rx: mpsc::Receiver<FileChange>,
        is_running: Arc<RwLock<bool>>,
    ) {
        let mut debounce_buffer = HashMap::new();
        let mut debounce_interval = interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = debounce_interval.tick() => {
                    // Process debounced changes
                    if !debounce_buffer.is_empty() {
                        let changes: Vec<_> = debounce_buffer.drain().collect();
                        for (_, change) in changes {
                            let mut graph_guard = graph.write().await;
                            let mut hashes_guard = file_hashes.write().await;

                            match change {
                                FileChange::Created(path) | FileChange::Modified(path) => {
                                    if let Err(e) = Self::index_single_file_static(
                                        &mut graph_guard,
                                        &mut hashes_guard,
                                        &config,
                                        &path,
                                    ).await {
                                        warn!("Failed to index {}: {}", path.display(), e);
                                    }
                                }
                                FileChange::Deleted(path) => {
                                    if hashes_guard.remove(&path).is_some() {
                                        // Find and remove file from graph
                                        if let Some(file_id) = graph_guard.file_by_path.get(&path).copied() {
                                            Self::remove_file_data_static(&mut graph_guard, file_id);
                                        }
                                    }
                                }
                                FileChange::Renamed { from, to } => {
                                    // Handle as delete + create
                                    if hashes_guard.remove(&from).is_some() {
                                        if let Some(file_id) = graph_guard.file_by_path.get(&from).copied() {
                                            Self::remove_file_data_static(&mut graph_guard, file_id);
                                        }
                                    }
                                    if let Err(e) = Self::index_single_file_static(
                                        &mut graph_guard,
                                        &mut hashes_guard,
                                        &config,
                                        &to,
                                    ).await {
                                        warn!("Failed to index renamed file {}: {}", to.display(), e);
                                    }
                                }
                            }

                            graph_guard.build_indexes();
                        }

                        // Sync to memory
                        if let Some(mem) = &memory {
                            let graph_clone = graph.read().await.clone();
                            let _ = Self::sync_to_memory_static(&graph_clone, mem, &scope).await;
                        }
                    }
                }
                change = rx.recv() => {
                    match change {
                        Some(change) => {
                            // Debounce: only keep latest change per file
                            let key = change.path_key();
                            debounce_buffer.insert(key, change);
                        }
                        None => break, // Channel closed
                    }
                }
                else => break,
            }

            if !*is_running.read().await {
                break;
            }
        }
    }

    async fn index_single_file_static(
        graph: &mut CodeGraph,
        file_hashes: &mut HashMap<PathBuf, String>,
        _config: &IndexerConfig,
        file_path: &Path,
    ) -> IndexerResult<()> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let hash = compute_hash(&content);

        if file_hashes.get(file_path) == Some(&hash) {
            return Ok(());
        }

        let lang = Language::from_path(file_path);
        let indexer = create_indexer(lang);

        let indexed_file = indexer.index_file(file_path, &content)?;

        // Remove old
        if graph.files.contains_key(&indexed_file.id) {
            Self::remove_file_data_static(graph, indexed_file.id);
        }

        let _file_id = graph.add_file(indexed_file.clone());
        file_hashes.insert(file_path.to_path_buf(), hash);

        Ok(())
    }

    fn remove_file_data_static(graph: &mut CodeGraph, file_id: FileId) {
        if let Some(file) = graph.files.get(&file_id) {
            let symbols: Vec<SymbolId> = file.symbols.iter().map(|s| s.id.clone()).collect();
            for symbol_id in &symbols {
                graph.symbols.remove(symbol_id);
                if let Some(name) = graph.symbols.get(symbol_id).map(|s| s.name.clone()) {
                    graph
                        .symbol_by_name
                        .entry(name)
                        .or_default()
                        .remove(symbol_id);
                }
            }
        }

        graph
            .calls
            .retain(|c| c.caller.file_id != file_id && c.callee.file_id != file_id);
        graph
            .imports
            .retain(|i| i.from_file != file_id && i.to_file != file_id);
        graph
            .coverage
            .retain(|c| c.test_symbol.file_id != file_id && c.covered_symbol.file_id != file_id);

        if let Some(file) = graph.files.get(&file_id) {
            let path = file.path.clone();
            graph.files.remove(&file_id);
            graph.file_by_path.remove(&path);
        }
    }

    async fn sync_to_memory_static(
        graph: &CodeGraph,
        memory: &Arc<MemoryService>,
        scope: &MemoryScope,
    ) -> IndexerResult<()> {
        use crate::graph::GraphSyncer;
        let syncer = GraphSyncer::new(memory.clone(), scope.clone());
        syncer.sync_graph(graph).await
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.replace("**", "*");
    let regex = pattern
        .replace('.', "\\.")
        .replace('*', ".*")
        .replace('?', ".");
    regex::Regex::new(&format!("^{regex}$")).is_ok_and(|re| re.is_match(text))
}

fn compute_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_incremental_indexer() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            src_dir.join("lib.rs"),
            r"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
",
        )
        .unwrap();

        let config = IndexerConfig {
            workspace_root: temp_dir.path().to_path_buf(),
            watch_enabled: false,
            ..Default::default()
        };

        let indexer = IncrementalIndexer::new(config).await.unwrap();
        let graph = indexer.full_reindex().await.unwrap();

        assert_eq!(graph.files.len(), 1);
        assert!(graph.symbols.values().any(|s| s.name == "add"));
        assert!(graph.symbols.values().any(|s| s.name == "test_add"));
    }
}
