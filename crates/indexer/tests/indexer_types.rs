use orbit_indexer::{FileId, IndexerConfig, IndexerError, Span, SymbolId, SymbolKind, Visibility};
use tempfile::TempDir;

#[test]
fn indexer_config_defaults() {
    let config = IndexerConfig::default();
    assert!(config.watch_enabled);
    assert!(config.index_tests);
    assert_eq!(config.max_file_size, 1024 * 1024);
}

#[test]
fn indexer_error_display() {
    let error = IndexerError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let display = format!("{error}");
    assert!(!display.is_empty());
}

#[test]
fn file_id_construction() {
    let file_id = FileId(42);
    assert_eq!(file_id.0, 42);
}

#[test]
fn symbol_id_construction() {
    let id = SymbolId {
        file_id: FileId(1),
        kind: SymbolKind::Function,
        name: "test_fn".to_string(),
        parent: None,
    };
    assert_eq!(id.name, "test_fn");
    assert_eq!(id.kind, SymbolKind::Function);
}

#[test]
fn symbol_id_with_parent() {
    let parent = SymbolId {
        file_id: FileId(1),
        kind: SymbolKind::Struct,
        name: "MyStruct".to_string(),
        parent: None,
    };
    let child = SymbolId {
        file_id: FileId(1),
        kind: SymbolKind::Method,
        name: "my_method".to_string(),
        parent: Some(Box::new(parent)),
    };
    assert_eq!(child.name, "my_method");
    assert!(child.parent.is_some());
}

#[test]
fn span_default() {
    let span = Span::default();
    assert_eq!(span.start_line, 0);
    assert_eq!(span.end_line, 0);
}

#[test]
fn span_construction() {
    let span = Span {
        file_id: FileId(1),
        start_line: 10,
        start_column: 1,
        end_line: 20,
        end_column: 5,
        start_offset: 200,
        end_offset: 500,
    };
    assert_eq!(span.start_line, 10);
    assert_eq!(span.end_line, 20);
}

#[test]
fn symbol_kind_variants() {
    assert_eq!(format!("{:?}", SymbolKind::Function), "Function");
    assert_eq!(format!("{:?}", SymbolKind::Class), "Class");
    assert_eq!(format!("{:?}", SymbolKind::Struct), "Struct");
    assert_eq!(format!("{:?}", SymbolKind::Enum), "Enum");
    assert_eq!(format!("{:?}", SymbolKind::Trait), "Trait");
}

#[test]
fn visibility_variants() {
    assert_eq!(format!("{:?}", Visibility::Public), "Public");
    assert_eq!(format!("{:?}", Visibility::Private), "Private");
    assert_eq!(format!("{:?}", Visibility::PubCrate), "PubCrate");
}

#[test]
fn indexer_workspace_root_config() {
    let temp_dir = TempDir::new().unwrap();
    let config = IndexerConfig {
        workspace_root: temp_dir.path().to_path_buf(),
        watch_enabled: false,
        ..Default::default()
    };
    assert!(!config.watch_enabled);
    assert_eq!(config.workspace_root, temp_dir.path());
}
