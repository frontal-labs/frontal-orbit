//! AST node types and symbol representations.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Unique identifier for a symbol in the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId {
    pub file_id: FileId,
    pub kind: SymbolKind,
    pub name: String,
    pub parent: Option<Box<SymbolId>>,
}

/// Unique identifier for a file in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct FileId(pub u64);

/// Kind of symbol (function, struct, enum, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    AsyncFunction,
    Method,
    Struct,
    Class,
    Enum,
    Trait,
    Impl,
    Module,
    Const,
    Static,
    TypeAlias,
    Macro,
    Field,
    Variant,
    Parameter,
    Local,
    Import,
    Export,
}

/// A symbol definition in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub name: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub span: Span,
    pub visibility: Visibility,
    pub attributes: Vec<String>,
    pub generics: Vec<String>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_const: bool,
}

/// Visibility of a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    PubCrate,
    PubSuper,
    PubIn(PathBuf),
}

/// Source code span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Span {
    pub file_id: FileId,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub start_offset: u32,
    pub end_offset: u32,
}

/// A reference to a symbol (call, use, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    pub symbol_id: SymbolId,
    pub kind: RefKind,
    pub span: Span,
    pub context: Option<String>, // e.g., calling function name
}

/// Kind of reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefKind {
    Call,
    MethodCall,
    FieldAccess,
    TypeAnnotation,
    Import,
    Use,
    PatternMatch,
    Attribute,
}

/// A file in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub id: FileId,
    pub path: PathBuf,
    pub language: Language,
    pub hash: String, // content hash for change detection
    pub symbols: Vec<Symbol>,
    pub references: Vec<SymbolRef>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub module_path: Vec<String>,
    pub last_modified: u64,
}

/// Programming language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    Unknown,
}

/// An import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub path: String,
    pub items: Vec<ImportItem>,
    pub span: Span,
    pub is_glob: bool,
}

/// An item in an import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
    pub is_self: bool,
}

/// An export statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub name: String,
    pub symbol_id: Option<SymbolId>,
    pub span: Span,
    pub reexport: Option<String>,
}

/// A function call edge in the call graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller: SymbolId,
    pub callee: SymbolId,
    pub call_site: Span,
    pub is_dynamic: bool,
    pub is_async: bool,
}

/// An import edge in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEdge {
    pub from_file: FileId,
    pub to_file: FileId,
    pub import: Import,
}

/// A test coverage edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageEdge {
    pub test_symbol: SymbolId,
    pub covered_symbol: SymbolId,
    pub coverage_type: CoverageType,
}

/// Type of test coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageType {
    Unit,
    Integration,
    Property,
    Fuzz,
}

/// Query result for callers of a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallersResult {
    pub symbol: Symbol,
    pub callers: Vec<CallEdge>,
    pub total_count: usize,
}

/// Query result for callees of a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalleesResult {
    pub symbol: Symbol,
    pub callees: Vec<CallEdge>,
    pub total_count: usize,
}

/// Query result for files touched by commits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesTouchedResult {
    pub files: Vec<IndexedFile>,
    pub commit_hashes: Vec<String>,
    pub since_commit: Option<String>,
}

/// Query result for tests covering a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestsCoveringResult {
    pub function: Symbol,
    pub tests: Vec<Symbol>,
    pub edges: Vec<CoverageEdge>,
}

/// Query result for diff since checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSinceResult {
    pub added: Vec<Symbol>,
    pub removed: Vec<Symbol>,
    pub modified: Vec<Symbol>,
    pub since_checkpoint: String,
}

impl Language {
    #[must_use]
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Language::Rust,
            Some("ts" | "tsx") => Language::TypeScript,
            Some("js" | "jsx") => Language::JavaScript,
            Some("py") => Language::Python,
            Some("go") => Language::Go,
            Some("java") => Language::Java,
            Some("cpp" | "cc" | "cxx" | "hpp" | "h") => Language::Cpp,
            Some("c") => Language::C,
            _ => Language::Unknown,
        }
    }

    #[must_use]
    pub fn supports_ast(&self) -> bool {
        matches!(
            self,
            Language::Rust | Language::TypeScript | Language::JavaScript | Language::Python
        )
    }
}
