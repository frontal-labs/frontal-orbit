//! Error types for the indexer.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for indexer operations.
pub type IndexerResult<T> = Result<T, IndexerError>;

/// Errors that can occur during indexing.
#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),

    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("Syn parse error: {0}")]
    SynParse(#[from] syn::Error),

    #[error("Tree-sitter error: {0}")]
    TreeSitter(String),

    #[error("Tree-sitter query error: {0}")]
    QueryError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("File too large: {path} ({size} bytes > {max} bytes)")]
    FileTooLarge {
        path: PathBuf,
        size: usize,
        max: usize,
    },

    #[error("Unsupported language for file: {path}")]
    UnsupportedLanguage { path: PathBuf },

    #[error("Indexer not initialized")]
    NotInitialized,

    #[error("Watcher already running")]
    WatcherAlreadyRunning,

    #[error("Watcher not running")]
    WatcherNotRunning,

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<String> for IndexerError {
    fn from(s: String) -> Self {
        IndexerError::Other(s)
    }
}

impl From<&str> for IndexerError {
    fn from(s: &str) -> Self {
        IndexerError::Other(s.to_string())
    }
}

impl From<tree_sitter::QueryError> for IndexerError {
    fn from(e: tree_sitter::QueryError) -> Self {
        IndexerError::QueryError(e.to_string())
    }
}

impl From<tree_sitter::LanguageError> for IndexerError {
    fn from(e: tree_sitter::LanguageError) -> Self {
        IndexerError::TreeSitter(e.to_string())
    }
}
