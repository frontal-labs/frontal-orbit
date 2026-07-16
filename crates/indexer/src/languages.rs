//! Language-specific AST parsing and symbol extraction.

pub mod rust;
pub mod tree_sitter;

use crate::{IndexedFile, IndexerResult, Language};
use std::path::Path;

/// Trait for language-specific indexers.
pub trait LanguageIndexer: Send + Sync {
    /// Index a single file and return the indexed file.
    fn index_file(&self, file_path: &Path, content: &str) -> IndexerResult<IndexedFile>;

    /// Get the language this indexer handles.
    fn language(&self) -> Language;

    /// Get file extensions this indexer handles.
    fn extensions(&self) -> &[&str];

    /// Check if this indexer can handle a file.
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| self.extensions().contains(&ext))
    }
}

/// Factory for creating language indexers.
#[must_use]
pub fn create_indexer(language: Language) -> Box<dyn LanguageIndexer> {
    match language {
        Language::Rust => Box::new(rust::RustIndexer::new()),
        Language::TypeScript => Box::new(tree_sitter::TreeSitterIndexer::new_typescript()),
        Language::JavaScript => Box::new(tree_sitter::TreeSitterIndexer::new_javascript()),
        Language::Python => Box::new(tree_sitter::TreeSitterIndexer::new_python()),
        Language::Go => Box::new(tree_sitter::TreeSitterIndexer::new_go()),
        Language::Java => Box::new(tree_sitter::TreeSitterIndexer::new_java()),
        Language::Cpp => Box::new(tree_sitter::TreeSitterIndexer::new_cpp()),
        Language::C => Box::new(tree_sitter::TreeSitterIndexer::new_c()),
        Language::Unknown => Box::new(tree_sitter::TreeSitterIndexer::new_unknown()),
    }
}
