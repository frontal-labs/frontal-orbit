//! Tree-sitter based indexer for non-Rust languages.
#![allow(dead_code)]

use crate::error::IndexerError;
use crate::languages::LanguageIndexer;
use crate::{FileId, IndexedFile, IndexerResult, Language, Span};
use std::collections::HashMap;
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language as TsLanguage, Node, Parser, Query, QueryCursor, Tree};

/// Tree-sitter based indexer for various languages.
pub struct TreeSitterIndexer {
    language: TsLanguage,
    language_type: Language,
    parser: Parser,
    queries: HashMap<String, Query>,
}

impl TreeSitterIndexer {
    pub fn new(language: TsLanguage, language_type: Language) -> IndexerResult<Self> {
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        Ok(Self {
            language,
            language_type,
            parser,
            queries: HashMap::new(),
        })
    }

    pub fn new_typescript() -> Self {
        let lang = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        Self::new(lang, Language::TypeScript).unwrap()
    }

    pub fn new_javascript() -> Self {
        let lang = tree_sitter_javascript::LANGUAGE.into();
        Self::new(lang, Language::JavaScript).unwrap()
    }

    pub fn new_python() -> Self {
        let lang = tree_sitter_python::LANGUAGE.into();
        Self::new(lang, Language::Python).unwrap()
    }

    pub fn new_go() -> Self {
        let lang = tree_sitter_go::LANGUAGE.into();
        Self::new(lang, Language::Go).unwrap()
    }

    pub fn new_java() -> Self {
        let lang = tree_sitter_java::LANGUAGE.into();
        Self::new(lang, Language::Java).unwrap()
    }

    pub fn new_cpp() -> Self {
        let lang = tree_sitter_cpp::LANGUAGE.into();
        Self::new(lang, Language::Cpp).unwrap()
    }

    pub fn new_c() -> Self {
        let lang = tree_sitter_c::LANGUAGE.into();
        Self::new(lang, Language::C).unwrap()
    }

    pub fn new_unknown() -> Self {
        let lang = tree_sitter_cpp::LANGUAGE.into();
        Self::new(lang, Language::Unknown).unwrap()
    }

    fn get_query(&mut self, name: &str) -> IndexerResult<&Query> {
        if !self.queries.contains_key(name) {
            let query_str = self.get_query_source(name);
            let query = Query::new(&self.language, query_str)?;
            self.queries.insert(name.to_string(), query);
        }
        Ok(self.queries.get(name).unwrap())
    }

    fn get_query_source(&self, name: &str) -> &str {
        match (self.language_type, name) {
            (Language::TypeScript | Language::JavaScript, "functions") => TS_FUNCTION_QUERY,
            (Language::TypeScript | Language::JavaScript, "classes") => TS_CLASS_QUERY,
            (Language::TypeScript | Language::JavaScript, "imports") => TS_IMPORT_QUERY,
            (Language::TypeScript | Language::JavaScript, "calls") => TS_CALL_QUERY,
            (Language::Python, "functions") => PY_FUNCTION_QUERY,
            (Language::Python, "classes") => PY_CLASS_QUERY,
            (Language::Python, "imports") => PY_IMPORT_QUERY,
            (Language::Python, "calls") => PY_CALL_QUERY,
            (Language::Go, "functions") => GO_FUNCTION_QUERY,
            (Language::Go, "imports") => GO_IMPORT_QUERY,
            (Language::Java, "functions") => JAVA_FUNCTION_QUERY,
            (Language::Java, "classes") => JAVA_CLASS_QUERY,
            (Language::Java, "imports") => JAVA_IMPORT_QUERY,
            (Language::Cpp | Language::C, "functions") => CPP_FUNCTION_QUERY,
            (Language::Cpp | Language::C, "classes") => CPP_CLASS_QUERY,
            (Language::Cpp | Language::C, "imports") => CPP_INCLUDE_QUERY,
            _ => "",
        }
    }

    fn parse_file(&mut self, content: &str) -> IndexerResult<Tree> {
        self.parser
            .parse(content, None)
            .ok_or_else(|| IndexerError::ParseError("Failed to parse file".to_string()))
    }

    fn run_query(
        &mut self,
        tree: &Tree,
        content: &str,
        query_name: &str,
    ) -> IndexerResult<Vec<(String, usize, usize)>> {
        let query = self.get_query(query_name)?;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, tree.root_node(), content.as_bytes());
        let mut results = Vec::new();
        while let Some(m) = matches.next() {
            for capture in m.captures {
                if capture.index == 0 {
                    let node = capture.node;
                    let name = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                    let start_line = node.start_position().row + 1;
                    let end_line = node.end_position().row + 1;
                    results.push((name, start_line, end_line));
                }
            }
        }
        Ok(results)
    }
}

impl LanguageIndexer for TreeSitterIndexer {
    fn index_file(&self, _file_path: &Path, _content: &str) -> IndexerResult<IndexedFile> {
        // Disabled - Tree-sitter indexer has complex lifetime issues
        // Use the Rust indexer for now
        Err(IndexerError::Other(
            "Tree-sitter indexer disabled - use Rust indexer".to_string(),
        ))
    }

    fn language(&self) -> Language {
        self.language_type
    }

    fn extensions(&self) -> &[&str] {
        static EMPTY: &[&str] = &[];
        EMPTY
    }
}

// TreeSitterIndexer cannot be cloned due to tree_sitter::Language not implementing Clone.
// The index_file method creates a new instance instead.

#[allow(clippy::cast_possible_truncation)]
fn node_to_span(file_id: FileId, node: &Node) -> Span {
    Span {
        file_id,
        start_line: node.start_position().row as u32 + 1,
        start_column: node.start_position().column as u32,
        end_line: node.end_position().row as u32 + 1,
        end_column: node.end_position().column as u32,
        start_offset: node.start_byte() as u32,
        end_offset: node.end_byte() as u32,
    }
}

fn compute_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// Tree-sitter queries for each language

const TS_FUNCTION_QUERY: &str = r"
(function_declaration name: (identifier) @name)
(arrow_function)
(method_definition name: (property_identifier) @name)
";

const TS_CLASS_QUERY: &str = r"
(class_declaration name: (type_identifier) @name)
(interface_declaration name: (type_identifier) @name)
";

const TS_IMPORT_QUERY: &str = r"
(import_statement source: (string) @path)
(import_specifier name: (identifier) @name)
";

const TS_CALL_QUERY: &str = r"
(call_expression function: (identifier) @name)
(member_expression property: (property_identifier) @name)
";

const PY_FUNCTION_QUERY: &str = r"
(function_definition name: (identifier) @name)
(async_function_definition name: (identifier) @name)
";

const PY_CLASS_QUERY: &str = r"
(class_definition name: (identifier) @name)
";

const PY_IMPORT_QUERY: &str = r"
(import_statement name: (dotted_name) @path)
(import_from_statement module_name: (dotted_name) @path)
";

const PY_CALL_QUERY: &str = r"
(call function: (identifier) @name)
(call function: (attribute attribute: (identifier) @name))
";

const GO_FUNCTION_QUERY: &str = r"
(function_declaration name: (identifier) @name)
(method_declaration name: (field_identifier) @name)
";

const GO_IMPORT_QUERY: &str = r"
(import_spec path: (interpreted_string_literal) @path)
(import_spec_list (import_spec path: (interpreted_string_literal) @path))
";

const JAVA_FUNCTION_QUERY: &str = r"
(method_declaration name: (identifier) @name)
(constructor_declaration name: (identifier) @name)
";

const JAVA_CLASS_QUERY: &str = r"
(class_declaration name: (identifier) @name)
(interface_declaration name: (identifier) @name)
";

const JAVA_IMPORT_QUERY: &str = r"
(import_declaration name: (scoped_identifier) @path)
";

const CPP_FUNCTION_QUERY: &str = r"
(function_declarator declarator: (identifier) @name)
(function_definition declarator: (function_declarator declarator: (identifier) @name))
";

const CPP_CLASS_QUERY: &str = r"
(class_specifier name: (type_identifier) @name)
(struct_specifier name: (type_identifier) @name)
";

const CPP_INCLUDE_QUERY: &str = r"
(preproc_include path: (string_literal) @path)
";
