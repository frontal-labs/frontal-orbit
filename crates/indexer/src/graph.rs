//! Code graph data structures and query API.

use crate::ast::{
    CallEdge, CalleesResult, CallersResult, CoverageEdge, DiffSinceResult, FileId,
    FilesTouchedResult, ImportEdge, IndexedFile, Span, Symbol, SymbolId, SymbolKind,
    TestsCoveringResult, Visibility,
};
use crate::error::IndexerResult;
use orbit_memory::{KgEntity, KgRelation, MemoryScope, MemoryService};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// The main code graph storing all indexed information.
#[derive(Debug, Default, Clone)]
pub struct CodeGraph {
    /// Files indexed, keyed by `FileId`.
    pub files: HashMap<FileId, IndexedFile>,

    /// Symbols indexed, keyed by `SymbolId`.
    pub symbols: HashMap<SymbolId, Symbol>,

    /// Call edges (caller -> callee).
    pub calls: Vec<CallEdge>,

    /// Import edges (file -> file).
    pub imports: Vec<ImportEdge>,

    /// Test coverage edges.
    pub coverage: Vec<CoverageEdge>,

    /// Reverse index: symbol name -> set of `SymbolIds`.
    pub symbol_by_name: HashMap<String, HashSet<SymbolId>>,

    /// Reverse index: file path -> `FileId`.
    pub file_by_path: HashMap<PathBuf, FileId>,

    /// Call graph adjacency: caller `SymbolId` -> list of callee `SymbolIds`.
    pub call_graph: HashMap<SymbolId, Vec<SymbolId>>,

    /// Reverse call graph: callee `SymbolId` -> list of caller `SymbolIds`.
    pub reverse_call_graph: HashMap<SymbolId, Vec<SymbolId>>,

    /// File dependency graph: file -> imported files.
    pub file_deps: HashMap<FileId, Vec<FileId>>,

    /// File reverse dependency graph: file -> files that import it.
    pub reverse_file_deps: HashMap<FileId, Vec<FileId>>,
}

impl CodeGraph {
    /// Create a new empty code graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file to the graph.
    pub fn add_file(&mut self, file: IndexedFile) -> FileId {
        let file_id = file.id;
        self.file_by_path.insert(file.path.clone(), file_id);

        for symbol in &file.symbols {
            self.symbol_by_name
                .entry(symbol.name.clone())
                .or_default()
                .insert(symbol.id.clone());
            self.symbols.insert(symbol.id.clone(), symbol.clone());
        }

        self.files.insert(file_id, file);
        file_id
    }

    /// Add a symbol to the graph.
    pub fn add_symbol(&mut self, symbol: Symbol) {
        let symbol_id = symbol.id.clone();
        self.symbol_by_name
            .entry(symbol.name.clone())
            .or_default()
            .insert(symbol_id.clone());
        self.symbols.insert(symbol_id, symbol);
    }

    /// Add a call edge.
    pub fn add_call(&mut self, edge: CallEdge) {
        self.call_graph
            .entry(edge.caller.clone())
            .or_default()
            .push(edge.callee.clone());
        self.reverse_call_graph
            .entry(edge.callee.clone())
            .or_default()
            .push(edge.caller.clone());
        self.calls.push(edge);
    }

    /// Add an import edge.
    pub fn add_import(&mut self, edge: ImportEdge) {
        self.file_deps
            .entry(edge.from_file)
            .or_default()
            .push(edge.to_file);
        self.reverse_file_deps
            .entry(edge.to_file)
            .or_default()
            .push(edge.from_file);
        self.imports.push(edge);
    }

    /// Add a coverage edge.
    pub fn add_coverage(&mut self, edge: CoverageEdge) {
        self.coverage.push(edge);
    }

    /// Get a file by path.
    #[must_use]
    pub fn get_file_by_path(&self, path: &PathBuf) -> Option<&IndexedFile> {
        self.file_by_path
            .get(path)
            .and_then(|id| self.files.get(id))
    }

    /// Get a symbol by ID.
    #[must_use]
    pub fn get_symbol(&self, id: &SymbolId) -> Option<&Symbol> {
        self.symbols.get(id)
    }

    /// Find symbols by name (fuzzy).
    #[must_use]
    pub fn find_symbols_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.symbol_by_name
            .get(name)
            .map(|ids| ids.iter().filter_map(|id| self.symbols.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get callers of a symbol.
    #[must_use]
    pub fn get_callers(&self, symbol_id: &SymbolId) -> Vec<&CallEdge> {
        self.reverse_call_graph
            .get(symbol_id)
            .map(|callers| {
                callers
                    .iter()
                    .filter_map(|caller| {
                        self.calls
                            .iter()
                            .find(|c| &c.caller == caller && &c.callee == symbol_id)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get callees of a symbol.
    #[must_use]
    pub fn get_callees(&self, symbol_id: &SymbolId) -> Vec<&CallEdge> {
        self.call_graph
            .get(symbol_id)
            .map(|callees| {
                callees
                    .iter()
                    .filter_map(|callee| {
                        self.calls
                            .iter()
                            .find(|c| &c.caller == symbol_id && &c.callee == callee)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get tests covering a symbol.
    #[must_use]
    pub fn get_tests_covering(&self, symbol_id: &SymbolId) -> Vec<&CoverageEdge> {
        self.coverage
            .iter()
            .filter(|e| &e.covered_symbol == symbol_id)
            .collect()
    }

    /// Get file dependencies.
    #[must_use]
    pub fn get_file_deps(&self, file_id: &FileId) -> Vec<&IndexedFile> {
        self.file_deps
            .get(file_id)
            .map(|deps| deps.iter().filter_map(|id| self.files.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get reverse file dependencies.
    #[must_use]
    pub fn get_reverse_file_deps(&self, file_id: &FileId) -> Vec<&IndexedFile> {
        self.reverse_file_deps
            .get(file_id)
            .map(|deps| deps.iter().filter_map(|id| self.files.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all symbols in a file.
    #[must_use]
    pub fn get_symbols_in_file(&self, file_id: &FileId) -> Vec<&Symbol> {
        self.files
            .get(file_id)
            .map(|f| f.symbols.iter().collect())
            .unwrap_or_default()
    }

    /// Get all functions in the codebase.
    #[must_use]
    pub fn get_all_functions(&self) -> Vec<&Symbol> {
        self.symbols
            .values()
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Function | SymbolKind::AsyncFunction | SymbolKind::Method
                )
            })
            .collect()
    }

    /// Get all test functions.
    #[must_use]
    pub fn get_test_functions(&self) -> Vec<&Symbol> {
        self.symbols
            .values()
            .filter(|s| {
                matches!(s.kind, SymbolKind::Function | SymbolKind::AsyncFunction)
                    && (s.name.starts_with("test_")
                        || s.attributes.iter().any(|a| a.contains("test")))
            })
            .collect()
    }

    /// Get statistics about the graph.
    #[must_use]
    pub fn stats(&self) -> GraphStats {
        GraphStats {
            file_count: self.files.len(),
            symbol_count: self.symbols.len(),
            call_edge_count: self.calls.len(),
            import_edge_count: self.imports.len(),
            coverage_edge_count: self.coverage.len(),
            function_count: self.get_all_functions().len(),
            test_function_count: self.get_test_functions().len(),
        }
    }

    /// Build derived indexes (call graph, file deps, reverse indexes).
    /// These are maintained incrementally by `add_*` methods, so this is a no-op.
    pub fn build_indexes(&mut self) {}

    /// Clear all data.
    pub fn clear(&mut self) {
        self.files.clear();
        self.symbols.clear();
        self.calls.clear();
        self.imports.clear();
        self.coverage.clear();
        self.symbol_by_name.clear();
        self.file_by_path.clear();
        self.call_graph.clear();
        self.reverse_call_graph.clear();
        self.file_deps.clear();
        self.reverse_file_deps.clear();
    }
}

/// Statistics about the code graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub file_count: usize,
    pub symbol_count: usize,
    pub call_edge_count: usize,
    pub import_edge_count: usize,
    pub coverage_edge_count: usize,
    pub function_count: usize,
    pub test_function_count: usize,
}

/// Query API for the code graph.
pub struct GraphQuery<'a> {
    graph: &'a CodeGraph,
    #[allow(dead_code)]
    memory: Option<&'a MemoryService>,
    #[allow(dead_code)]
    scope: MemoryScope,
}

impl<'a> GraphQuery<'a> {
    /// Create a new query with just the in-memory graph.
    #[must_use]
    pub fn new(graph: &'a CodeGraph) -> Self {
        Self {
            graph,
            memory: None,
            scope: MemoryScope::default(),
        }
    }

    /// Create a new query with memory backend for persistent queries.
    #[must_use]
    pub fn with_memory(
        graph: &'a CodeGraph,
        memory: &'a MemoryService,
        scope: MemoryScope,
    ) -> Self {
        Self {
            graph,
            memory: Some(memory),
            scope,
        }
    }

    /// Find callers of a symbol by name.
    #[must_use]
    pub fn callers_of(&self, symbol_name: &str) -> CallersResult {
        let symbols = self.graph.find_symbols_by_name(symbol_name);
        let mut all_callers = Vec::new();

        for symbol in &symbols {
            let callers = self.graph.get_callers(&symbol.id);
            all_callers.extend(callers.into_iter().cloned());
        }

        let total_count = all_callers.len();
        CallersResult {
            symbol: symbols.first().copied().cloned().unwrap_or_default(),
            callers: all_callers,
            total_count,
        }
    }

    /// Find callees of a symbol by name.
    #[must_use]
    pub fn callees_of(&self, symbol_name: &str) -> CalleesResult {
        let symbols = self.graph.find_symbols_by_name(symbol_name);
        let mut all_callees = Vec::new();

        for symbol in &symbols {
            let callees = self.graph.get_callees(&symbol.id);
            all_callees.extend(callees.into_iter().cloned());
        }

        let total_count = all_callees.len();
        CalleesResult {
            symbol: symbols.first().copied().cloned().unwrap_or_default(),
            callees: all_callees,
            total_count,
        }
    }

    /// Find tests covering a function by name.
    #[must_use]
    pub fn tests_covering(&self, function_name: &str) -> TestsCoveringResult {
        let functions = self.graph.find_symbols_by_name(function_name);
        let mut all_tests = Vec::new();
        let mut all_edges = Vec::new();

        for func in &functions {
            if matches!(
                func.kind,
                SymbolKind::Function | SymbolKind::AsyncFunction | SymbolKind::Method
            ) {
                let edges = self.graph.get_tests_covering(&func.id);
                for edge in &edges {
                    if let Some(test) = self.graph.get_symbol(&edge.test_symbol) {
                        all_tests.push((*test).clone());
                        all_edges.push((*edge).clone());
                    }
                }
            }
        }

        TestsCoveringResult {
            function: functions.first().copied().cloned().unwrap_or_default(),
            tests: all_tests,
            edges: all_edges,
        }
    }

    /// Get files modified since a checkpoint (requires git integration).
    /// This is a placeholder - actual implementation would use git.
    #[must_use]
    pub fn files_modified_since(&self, since_commit: &str) -> FilesTouchedResult {
        FilesTouchedResult {
            files: vec![],
            commit_hashes: vec![],
            since_commit: Some(since_commit.to_string()),
        }
    }

    /// Get diff since a checkpoint.
    #[must_use]
    pub fn diff_since_checkpoint(&self, checkpoint: &str) -> DiffSinceResult {
        DiffSinceResult {
            added: vec![],
            removed: vec![],
            modified: vec![],
            since_checkpoint: checkpoint.to_string(),
        }
    }

    /// Get dependencies of a file.
    #[must_use]
    pub fn deps_of(&self, file_path: &PathBuf) -> Vec<&IndexedFile> {
        self.graph
            .get_file_by_path(file_path)
            .map(|f| self.graph.get_file_deps(&f.id))
            .unwrap_or_default()
    }

    /// Get reverse dependencies of a file (files that import it).
    #[must_use]
    pub fn reverse_deps_of(&self, file_path: &PathBuf) -> Vec<&IndexedFile> {
        self.graph
            .get_file_by_path(file_path)
            .map(|f| self.graph.get_reverse_file_deps(&f.id))
            .unwrap_or_default()
    }

    /// Get all symbols in a file.
    #[must_use]
    pub fn symbols_in_file(&self, file_path: &PathBuf) -> Vec<&Symbol> {
        self.graph
            .get_file_by_path(file_path)
            .map(|f| self.graph.get_symbols_in_file(&f.id))
            .unwrap_or_default()
    }

    /// Search symbols by name pattern.
    #[must_use]
    pub fn search_symbols(&self, pattern: &str) -> Vec<&Symbol> {
        self.graph
            .symbols
            .values()
            .filter(|s| s.name.contains(pattern))
            .collect()
    }

    /// Get graph statistics.
    #[must_use]
    pub fn stats(&self) -> GraphStats {
        self.graph.stats()
    }
}

/// Wrapper to sync `CodeGraph` with `MemoryService` (Neo4j/Pinecone).
pub struct GraphSyncer {
    memory: Arc<MemoryService>,
    scope: MemoryScope,
}

impl GraphSyncer {
    #[must_use]
    pub fn new(memory: Arc<MemoryService>, scope: MemoryScope) -> Self {
        Self { memory, scope }
    }

    /// Sync the entire graph to the memory backend.
    #[allow(clippy::unused_async)]
    pub async fn sync_graph(&self, graph: &CodeGraph) -> IndexerResult<()> {
        // Upsert file entities
        for file in graph.files.values() {
            let entity = KgEntity {
                id: format!("file:{}", file.id.0),
                label: file.path.display().to_string(),
                entity_type: "file".to_string(),
            };
            self.memory.upsert_entity(&self.scope, entity);

            // Upsert symbols as entities
            for symbol in &file.symbols {
                let entity = KgEntity {
                    id: format!("symbol:{}", symbol.id.name),
                    label: symbol.name.clone(),
                    entity_type: format!("{:?}", symbol.kind).to_lowercase(),
                };
                self.memory.upsert_entity(&self.scope, entity);

                // Add relation: file -> contains -> symbol
                let relation = KgRelation {
                    subject_id: format!("file:{}", file.id.0),
                    predicate: "contains".to_string(),
                    object_id: format!("symbol:{}", symbol.id.name),
                };
                self.memory.add_relation(&self.scope, relation);
            }
        }

        // Upsert call edges as relations
        for call in &graph.calls {
            let relation = KgRelation {
                subject_id: format!("symbol:{}", call.caller.name),
                predicate: "calls".to_string(),
                object_id: format!("symbol:{}", call.callee.name),
            };
            self.memory.add_relation(&self.scope, relation);
        }

        // Upsert import edges
        for import in &graph.imports {
            let relation = KgRelation {
                subject_id: format!("file:{}", import.from_file.0),
                predicate: "imports".to_string(),
                object_id: format!("file:{}", import.to_file.0),
            };
            self.memory.add_relation(&self.scope, relation);
        }

        // Upsert coverage edges
        for coverage in &graph.coverage {
            let relation = KgRelation {
                subject_id: format!("symbol:{}", coverage.test_symbol.name),
                predicate: "covers".to_string(),
                object_id: format!("symbol:{}", coverage.covered_symbol.name),
            };
            self.memory.add_relation(&self.scope, relation);
        }

        Ok(())
    }
}

impl Default for Symbol {
    fn default() -> Self {
        Self {
            id: SymbolId {
                file_id: FileId(0),
                kind: SymbolKind::Function,
                name: String::new(),
                parent: None,
            },
            kind: SymbolKind::Function,
            name: String::new(),
            signature: None,
            doc_comment: None,
            span: Span {
                file_id: FileId(0),
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 0,
                start_offset: 0,
                end_offset: 0,
            },
            visibility: Visibility::Private,
            attributes: vec![],
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileId, IndexedFile, Language, Span, Symbol, SymbolId, SymbolKind, Visibility};

    #[test]
    fn test_code_graph_add_file() {
        let mut graph = CodeGraph::new();
        let file = IndexedFile {
            id: FileId(1),
            path: PathBuf::from("src/lib.rs"),
            language: Language::Rust,
            hash: "abc123".to_string(),
            symbols: vec![],
            references: vec![],
            imports: vec![],
            exports: vec![],
            module_path: vec!["crate".to_string()],
            last_modified: 1_234_567_890,
        };

        let file_id = graph.add_file(file);
        assert_eq!(file_id, FileId(1));
        assert_eq!(graph.files.len(), 1);
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_code_graph_call_graph() {
        let mut graph = CodeGraph::new();

        let caller_sym_id = SymbolId {
            file_id: FileId(1),
            kind: SymbolKind::Function,
            name: "caller".to_string(),
            parent: None,
        };
        let callee_sym_id = SymbolId {
            file_id: FileId(1),
            kind: SymbolKind::Function,
            name: "callee".to_string(),
            parent: None,
        };

        let caller_sym = Symbol {
            id: caller_sym_id.clone(),
            kind: SymbolKind::Function,
            name: "caller".to_string(),
            signature: None,
            doc_comment: None,
            span: Span::default(),
            visibility: Visibility::Public,
            attributes: vec![],
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };
        let callee_sym = Symbol {
            id: callee_sym_id.clone(),
            kind: SymbolKind::Function,
            name: "callee".to_string(),
            signature: None,
            doc_comment: None,
            span: Span::default(),
            visibility: Visibility::Public,
            attributes: vec![],
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };

        graph.add_symbol(caller_sym);
        graph.add_symbol(callee_sym);

        let edge = CallEdge {
            caller: caller_sym_id.clone(),
            callee: callee_sym_id.clone(),
            call_site: Span::default(),
            is_dynamic: false,
            is_async: false,
        };
        graph.add_call(edge);

        let callers = graph.get_callers(&callee_sym_id);
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].caller.name, "caller");

        let callees = graph.get_callees(&caller_sym_id);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].callee.name, "callee");
    }

    #[test]
    fn test_graph_query() {
        let mut graph = CodeGraph::new();
        let file_id = FileId(1);
        let symbol_id = SymbolId {
            file_id,
            kind: SymbolKind::Function,
            name: "test_fn".to_string(),
            parent: None,
        };

        let symbol = Symbol {
            id: symbol_id.clone(),
            kind: SymbolKind::Function,
            name: "test_fn".to_string(),
            signature: Some("fn test_fn()".to_string()),
            doc_comment: None,
            span: Span::default(),
            visibility: Visibility::Public,
            attributes: vec!["test".to_string()],
            generics: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
        };

        graph.add_symbol(symbol);

        let query = GraphQuery::new(&graph);
        let results = query.search_symbols("test_fn");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_fn");
    }
}
