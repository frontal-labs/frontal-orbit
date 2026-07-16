//! Configuration for the indexer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration for the indexer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Root directory of the workspace to index.
    pub workspace_root: PathBuf,

    /// Whether to enable file watching for incremental updates.
    #[serde(default = "default_true")]
    pub watch_enabled: bool,

    /// Maximum file size to index (bytes).
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,

    /// Languages to index with their configurations.
    #[serde(default)]
    pub languages: HashMap<String, LanguageConfig>,

    /// Patterns to exclude from indexing (glob patterns).
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Whether to index test files.
    #[serde(default = "default_true")]
    pub index_tests: bool,

    /// Whether to generate embeddings for symbols.
    #[serde(default = "default_false")]
    pub generate_embeddings: bool,

    /// Number of worker threads for parallel indexing.
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_max_file_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_workers() -> usize {
    std::thread::available_parallelism().map_or(4, std::num::NonZero::get)
}

impl Default for IndexerConfig {
    fn default() -> Self {
        let mut languages = HashMap::new();
        languages.insert("rust".to_string(), LanguageConfig::rust());
        languages.insert("typescript".to_string(), LanguageConfig::typescript());
        languages.insert("javascript".to_string(), LanguageConfig::javascript());
        languages.insert("python".to_string(), LanguageConfig::python());
        languages.insert("go".to_string(), LanguageConfig::go());
        languages.insert("java".to_string(), LanguageConfig::java());
        languages.insert("cpp".to_string(), LanguageConfig::cpp());

        Self {
            workspace_root: PathBuf::from("."),
            watch_enabled: true,
            max_file_size: default_max_file_size(),
            languages,
            exclude_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
            index_tests: true,
            generate_embeddings: false,
            workers: default_workers(),
        }
    }
}

/// Configuration for a specific language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// File extensions for this language.
    pub extensions: Vec<String>,

    /// Tree-sitter language name (if using tree-sitter).
    pub tree_sitter_name: Option<String>,

    /// Whether to use syn for Rust (more accurate).
    #[serde(default = "default_true")]
    pub use_syn_for_rust: bool,

    /// Custom queries for symbol extraction (tree-sitter query language).
    #[serde(default)]
    pub custom_queries: Vec<String>,
}

impl LanguageConfig {
    /// Default configuration for Rust.
    #[must_use]
    pub fn rust() -> Self {
        Self {
            extensions: vec!["rs".to_string()],
            tree_sitter_name: Some("rust".to_string()),
            use_syn_for_rust: true,
            custom_queries: vec![],
        }
    }

    /// Default configuration for TypeScript.
    #[must_use]
    pub fn typescript() -> Self {
        Self {
            extensions: vec!["ts".to_string(), "tsx".to_string()],
            tree_sitter_name: Some("typescript".to_string()),
            use_syn_for_rust: false,
            custom_queries: vec![],
        }
    }

    /// Default configuration for JavaScript.
    #[must_use]
    pub fn javascript() -> Self {
        Self {
            extensions: vec!["js".to_string(), "jsx".to_string()],
            tree_sitter_name: Some("javascript".to_string()),
            use_syn_for_rust: false,
            custom_queries: vec![],
        }
    }

    /// Default configuration for Python.
    #[must_use]
    pub fn python() -> Self {
        Self {
            extensions: vec!["py".to_string()],
            tree_sitter_name: Some("python".to_string()),
            use_syn_for_rust: false,
            custom_queries: vec![],
        }
    }

    /// Default configuration for Go.
    #[must_use]
    pub fn go() -> Self {
        Self {
            extensions: vec!["go".to_string()],
            tree_sitter_name: Some("go".to_string()),
            use_syn_for_rust: false,
            custom_queries: vec![],
        }
    }

    /// Default configuration for Java.
    #[must_use]
    pub fn java() -> Self {
        Self {
            extensions: vec!["java".to_string()],
            tree_sitter_name: Some("java".to_string()),
            use_syn_for_rust: false,
            custom_queries: vec![],
        }
    }

    /// Default configuration for C++.
    #[must_use]
    pub fn cpp() -> Self {
        Self {
            extensions: vec![
                "cpp".to_string(),
                "cc".to_string(),
                "cxx".to_string(),
                "h".to_string(),
                "hpp".to_string(),
            ],
            tree_sitter_name: Some("cpp".to_string()),
            use_syn_for_rust: false,
            custom_queries: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IndexerConfig::default();
        assert!(config.watch_enabled);
        assert_eq!(config.max_file_size, 1024 * 1024);
        assert!(config.languages.contains_key("rust"));
        assert!(config.languages.contains_key("typescript"));
        assert!(config.index_tests);
        assert!(!config.generate_embeddings);
    }

    #[test]
    fn test_language_configs() {
        let rust = LanguageConfig::rust();
        assert_eq!(rust.extensions, vec!["rs"]);
        assert!(rust.use_syn_for_rust);

        let ts = LanguageConfig::typescript();
        assert_eq!(ts.extensions, vec!["ts", "tsx"]);
    }
}
