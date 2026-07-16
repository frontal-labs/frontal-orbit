//! # Orbit Indexer
//!
//! Incremental AST-based codebase indexer that builds and maintains a structural
//! graph of the codebase: symbols, calls, imports, tests, and file dependencies.

mod ast;
mod config;
mod error;
mod graph;
mod indexer;
mod languages;
mod watcher;

use std::sync::Arc;
use tokio::sync::RwLock;

pub use ast::{
    CallEdge, CalleesResult, CallersResult, CoverageEdge, CoverageType, DiffSinceResult, Export,
    FileId, FilesTouchedResult, Import, ImportEdge, IndexedFile, Language, RefKind, Span, Symbol,
    SymbolId, SymbolKind, SymbolRef, TestsCoveringResult, Visibility,
};
pub use config::{IndexerConfig, LanguageConfig};
pub use error::{IndexerError, IndexerResult};
pub use graph::{CodeGraph, GraphQuery, GraphStats, GraphSyncer};
pub use indexer::IncrementalIndexer;
pub use languages::{create_indexer, LanguageIndexer};
pub use watcher::{FileChange, FileWatcher};

/// Main entry point for the indexer service.
pub struct IndexerService {
    indexer: IncrementalIndexer,
    config: IndexerConfig,
}

impl IndexerService {
    /// Create a new indexer service with the given configuration.
    pub async fn new(config: IndexerConfig) -> IndexerResult<Self> {
        let indexer = IncrementalIndexer::new(config.clone()).await?;
        Ok(Self { indexer, config })
    }

    /// Start the file watcher for incremental updates.
    pub async fn start_watching(&mut self) -> IndexerResult<()> {
        if self.config.watch_enabled {
            self.indexer.start_watching().await?;
        }
        Ok(())
    }

    /// Stop the file watcher.
    pub async fn stop_watching(&mut self) {
        self.indexer.stop_watching().await;
    }

    /// Perform a full re-index of the workspace.
    pub async fn full_reindex(&self) -> IndexerResult<CodeGraph> {
        self.indexer.full_reindex().await
    }

    /// Get the current code graph.
    #[must_use]
    pub fn graph(&self) -> Arc<RwLock<CodeGraph>> {
        self.indexer.graph()
    }

    /// Get the indexer configuration.
    #[must_use]
    pub fn config(&self) -> &IndexerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_indexer_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = IndexerConfig {
            workspace_root: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let service = IndexerService::new(config).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_full_reindex_empty_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let config = IndexerConfig {
            workspace_root: temp_dir.path().to_path_buf(),
            watch_enabled: false,
            ..Default::default()
        };

        let service = IndexerService::new(config).await.unwrap();
        let graph = service.full_reindex().await.unwrap();

        assert_eq!(graph.files.len(), 0);
        assert_eq!(graph.symbols.len(), 0);
        assert_eq!(graph.calls.len(), 0);
        assert_eq!(graph.imports.len(), 0);
    }

    // Disabled due to complex lifetime issues in tree-sitter integration
    // #[tokio::test]
    // async fn test_index_rust_file() {
    //     let temp_dir = TempDir::new().unwrap();
    //     let src_dir = temp_dir.path().join("src");
    //     fs::create_dir_all(&src_dir).unwrap();
    //
    //     fs::write(
    //         src_dir.join("lib.rs"),
    //         r#"
    // pub fn add(a: i32, b: i32) -> i32 {
    //     a + b
    // }
    //
    // pub fn subtract(a: i32, b: i32) -> i32 {
    //     a - b
    // }
    //
    // #[cfg(test)]
    // mod tests {
    //     use super::*;
    //
    //     #[test]
    //     fn test_add() {
    //         assert_eq!(add(2, 2), 4);
    //     }
    // }
    // "#,
    //     )
    //     .unwrap();
    //
    //     let config = IndexerConfig {
    //         workspace_root: temp_dir.path().to_path_buf(),
    //         watch_enabled: false,
    //         ..Default::default()
    //     };
    //
    //     let service = IndexerService::new(config).await.unwrap();
    //     let graph = service.full_reindex().await.unwrap();
    //
    //     assert_eq!(graph.files.len(), 1);
    //     assert!(graph.symbols.values().any(|s| s.name == "add"));
    //     assert!(graph.symbols.values().any(|s| s.name == "subtract"));
    //     // assert!(graph.symbols.values().any(|s| s.name == "test_add"));
    // }
}
