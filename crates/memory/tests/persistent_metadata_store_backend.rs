use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub use orbit_memory::{MemoryMetadata, MemoryMetadataStore, MemoryScope};

#[path = "../src/persistent_metadata_store.rs"]
mod persistent_metadata_store;

use persistent_metadata_store::PersistentFileMetadataStore;

fn temp_file_path(label: &str) -> PathBuf {
    // Timestamp alone collides when parallel tests read the same instant.
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let pid = std::process::id();
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("orbit-memory-{label}-{pid}-{stamp}-{serial}.tsv"))
}

#[test]
fn persisted_store_round_trips_items_across_instances() {
    let file = temp_file_path("roundtrip");
    let scope = MemoryScope::new("session-a");
    let metadata = MemoryMetadata {
        id: "mem-1".to_string(),
        source: "unit-test".to_string(),
        text: "persist me".to_string(),
        tags: vec!["a".to_string(), "b".to_string()],
        created_at_ms: 123,
        embedding_model: "local-hash-embedding-v1".to_string(),
        embedding_provider: "local-hash".to_string(),
        embedding_dimension: 384,
        embedding_revision: None,
    };

    {
        let store = PersistentFileMetadataStore::new(file.clone());
        store.upsert_item(&scope, metadata.clone());
        assert_eq!(store.count_items(&scope), 1);
    }

    let restored = PersistentFileMetadataStore::new(file.clone());
    let loaded = restored
        .get_item(&scope, "mem-1")
        .expect("item should persist");
    assert_eq!(loaded, metadata);

    let _ = fs::remove_file(file);
}

#[test]
fn persisted_store_isolates_by_scope() {
    let file = temp_file_path("scope");
    let scope_a = MemoryScope::new("session-a");
    let scope_b = MemoryScope::new("session-b");
    let store = PersistentFileMetadataStore::new(file.clone());

    store.upsert_item(
        &scope_a,
        MemoryMetadata {
            id: "a-1".to_string(),
            source: "scope-a".to_string(),
            text: "alpha".to_string(),
            tags: vec!["a".to_string()],
            created_at_ms: 1,
            embedding_model: "local-hash-embedding-v1".to_string(),
            embedding_provider: "local-hash".to_string(),
            embedding_dimension: 384,
            embedding_revision: None,
        },
    );
    store.upsert_item(
        &scope_b,
        MemoryMetadata {
            id: "b-1".to_string(),
            source: "scope-b".to_string(),
            text: "beta".to_string(),
            tags: vec!["b".to_string()],
            created_at_ms: 2,
            embedding_model: "local-hash-embedding-v1".to_string(),
            embedding_provider: "local-hash".to_string(),
            embedding_dimension: 384,
            embedding_revision: None,
        },
    );

    assert_eq!(store.count_items(&scope_a), 1);
    assert_eq!(store.count_items(&scope_b), 1);
    assert!(store.get_item(&scope_a, "b-1").is_none());
    assert!(store.get_item(&scope_b, "a-1").is_none());

    let _ = fs::remove_file(file);
}

#[test]
fn persisted_store_ignores_corrupt_lines() {
    let file = temp_file_path("corrupt");
    let contents = [
        "bad\\qline",
        "session-x\t\t\tmem-1\tsource\ttext\ttag1\u{1F}tag2\t99",
        "",
    ]
    .join("\n");
    fs::write(&file, contents).expect("write fixture");

    let scope = MemoryScope::new("session-x");
    let store = PersistentFileMetadataStore::new(file.clone());
    let loaded = store
        .get_item(&scope, "mem-1")
        .expect("valid line should load");
    assert_eq!(loaded.tags, vec!["tag1".to_string(), "tag2".to_string()]);
    assert_eq!(loaded.embedding_model, "unknown");
    assert_eq!(loaded.embedding_provider, "unknown");
    assert_eq!(loaded.embedding_dimension, 0);
    assert_eq!(store.count_items(&scope), 1);

    let _ = fs::remove_file(file);
}

#[test]
fn persisted_store_deletes_items_and_rewrites_snapshot() {
    let file = temp_file_path("delete");
    let scope = MemoryScope::new("session-a");
    let store = PersistentFileMetadataStore::new(file.clone());

    store.upsert_item(
        &scope,
        MemoryMetadata {
            id: "mem-1".to_string(),
            source: "unit-test".to_string(),
            text: "persist me".to_string(),
            tags: vec!["a".to_string()],
            created_at_ms: 123,
            embedding_model: "local-hash-embedding-v1".to_string(),
            embedding_provider: "local-hash".to_string(),
            embedding_dimension: 384,
            embedding_revision: None,
        },
    );

    assert!(store.delete_item(&scope, "mem-1"));
    assert_eq!(store.count_items(&scope), 0);

    let restored = PersistentFileMetadataStore::new(file.clone());
    assert!(restored.get_item(&scope, "mem-1").is_none());

    let _ = fs::remove_file(file);
}
