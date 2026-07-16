use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{MemoryMetadata, MemoryMetadataStore, MemoryScope};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ScopedMemoryKey {
    scope: MemoryScope,
    id: String,
}

impl ScopedMemoryKey {
    fn new(scope: &MemoryScope, id: impl Into<String>) -> Self {
        Self {
            scope: scope.clone(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedEntry {
    session_id: String,
    repo_id: Option<String>,
    branch_id: Option<String>,
    id: String,
    source: String,
    text: String,
    tags: Vec<String>,
    created_at_ms: u128,
    embedding_model: String,
    embedding_provider: String,
    embedding_dimension: usize,
    embedding_revision: Option<String>,
}

impl PersistedEntry {
    fn from_metadata(scope: &MemoryScope, metadata: &MemoryMetadata) -> Self {
        Self {
            session_id: scope.session_id.clone(),
            repo_id: scope.repo_id.clone(),
            branch_id: scope.branch_id.clone(),
            id: metadata.id.clone(),
            source: metadata.source.clone(),
            text: metadata.text.clone(),
            tags: metadata.tags.clone(),
            created_at_ms: metadata.created_at_ms,
            embedding_model: metadata.embedding_model.clone(),
            embedding_provider: metadata.embedding_provider.clone(),
            embedding_dimension: metadata.embedding_dimension,
            embedding_revision: metadata.embedding_revision.clone(),
        }
    }

    fn into_parts(self) -> (ScopedMemoryKey, MemoryMetadata) {
        let scope = MemoryScope {
            session_id: self.session_id,
            repo_id: self.repo_id,
            branch_id: self.branch_id,
        };
        let metadata = MemoryMetadata {
            id: self.id.clone(),
            source: self.source,
            text: self.text,
            tags: self.tags,
            created_at_ms: self.created_at_ms,
            embedding_model: self.embedding_model,
            embedding_provider: self.embedding_provider,
            embedding_dimension: self.embedding_dimension,
            embedding_revision: self.embedding_revision,
        };
        (ScopedMemoryKey::new(&scope, self.id), metadata)
    }

    fn encode_line(&self) -> String {
        let fields = [
            escape_field(&self.session_id),
            self.repo_id
                .as_deref()
                .map_or_else(String::new, escape_field),
            self.branch_id
                .as_deref()
                .map_or_else(String::new, escape_field),
            escape_field(&self.id),
            escape_field(&self.source),
            escape_field(&self.text),
            escape_field(&encode_tags(&self.tags)),
            self.created_at_ms.to_string(),
            escape_field(&self.embedding_model),
            escape_field(&self.embedding_provider),
            self.embedding_dimension.to_string(),
            self.embedding_revision
                .as_deref()
                .map_or_else(String::new, escape_field),
        ];
        fields.join("\t")
    }

    fn decode_line(line: &str) -> Option<Self> {
        let parts = split_escaped_tab_fields(line)?;
        match parts.len() {
            8 => {
                let created_at_ms = parts[7].parse::<u128>().ok()?;
                Some(Self {
                    session_id: parts[0].clone(),
                    repo_id: decode_optional(parts[1].clone()),
                    branch_id: decode_optional(parts[2].clone()),
                    id: parts[3].clone(),
                    source: parts[4].clone(),
                    text: parts[5].clone(),
                    tags: decode_tags(&parts[6]),
                    created_at_ms,
                    embedding_model: "unknown".to_string(),
                    embedding_provider: "unknown".to_string(),
                    embedding_dimension: 0,
                    embedding_revision: None,
                })
            }
            12 => {
                let created_at_ms = parts[7].parse::<u128>().ok()?;
                let embedding_dimension = parts[10].parse::<usize>().ok()?;
                Some(Self {
                    session_id: parts[0].clone(),
                    repo_id: decode_optional(parts[1].clone()),
                    branch_id: decode_optional(parts[2].clone()),
                    id: parts[3].clone(),
                    source: parts[4].clone(),
                    text: parts[5].clone(),
                    tags: decode_tags(&parts[6]),
                    created_at_ms,
                    embedding_model: parts[8].clone(),
                    embedding_provider: parts[9].clone(),
                    embedding_dimension,
                    embedding_revision: decode_optional(parts[11].clone()),
                })
            }
            _ => None,
        }
    }
}

/// File-backed metadata store using a line-based escaped TSV format.
///
/// The store keeps an in-memory index for fast reads and rewrites the
/// whole snapshot atomically on each upsert.
#[derive(Debug, Clone)]
pub struct PersistentFileMetadataStore {
    path: Arc<PathBuf>,
    inner: Arc<RwLock<BTreeMap<ScopedMemoryKey, MemoryMetadata>>>,
}

impl PersistentFileMetadataStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = load_snapshot(&path);
        Self {
            path: Arc::new(path),
            inner: Arc::new(RwLock::new(state)),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_ref().as_path()
    }
}

impl MemoryMetadataStore for PersistentFileMetadataStore {
    fn upsert_item(&self, scope: &MemoryScope, item: MemoryMetadata) {
        let key = ScopedMemoryKey::new(scope, item.id.clone());
        let snapshot = {
            let mut state = self.inner.write().expect("metadata store lock poisoned");
            state.insert(key, item);
            state.clone()
        };
        persist_snapshot(self.path(), &snapshot);
    }

    fn get_item(&self, scope: &MemoryScope, id: &str) -> Option<MemoryMetadata> {
        let key = ScopedMemoryKey::new(scope, id.to_string());
        let state = self.inner.read().expect("metadata store lock poisoned");
        state.get(&key).cloned()
    }

    fn list_items(&self, scope: &MemoryScope) -> Vec<MemoryMetadata> {
        let state = self.inner.read().expect("metadata store lock poisoned");
        state
            .iter()
            .filter(|(key, _)| key.scope == *scope)
            .map(|(_, value)| value.clone())
            .collect()
    }

    fn count_items(&self, scope: &MemoryScope) -> usize {
        let state = self.inner.read().expect("metadata store lock poisoned");
        state.keys().filter(|key| key.scope == *scope).count()
    }

    fn delete_item(&self, scope: &MemoryScope, id: &str) -> bool {
        let key = ScopedMemoryKey::new(scope, id.to_string());
        let removed = {
            let mut state = self.inner.write().expect("metadata store lock poisoned");
            state.remove(&key).is_some()
        };
        if removed {
            let snapshot = self
                .inner
                .read()
                .expect("metadata store lock poisoned")
                .clone();
            persist_snapshot(self.path(), &snapshot);
        }
        removed
    }
}

fn load_snapshot(path: &Path) -> BTreeMap<ScopedMemoryKey, MemoryMetadata> {
    let Ok(contents) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };

    let mut state = BTreeMap::new();
    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(entry) = PersistedEntry::decode_line(line) else {
            continue;
        };
        let (key, metadata) = entry.into_parts();
        state.insert(key, metadata);
    }
    state
}

fn persist_snapshot(path: &Path, state: &BTreeMap<ScopedMemoryKey, MemoryMetadata>) {
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    let mut lines = String::new();
    for (key, metadata) in state {
        let entry = PersistedEntry::from_metadata(&key.scope, metadata);
        lines.push_str(&entry.encode_line());
        lines.push('\n');
    }

    let temp_path = path.with_extension("tmp");
    if fs::write(&temp_path, lines).is_err() {
        return;
    }
    let _ = fs::rename(&temp_path, path);
}

fn decode_optional(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn encode_tags(tags: &[String]) -> String {
    tags.join("\u{1F}")
}

fn decode_tags(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split('\u{1F}').map(ToString::to_string).collect()
    }
}

fn escape_field(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn split_escaped_tab_fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                let escaped = chars.next()?;
                match escaped {
                    '\\' => current.push('\\'),
                    't' => current.push('\t'),
                    'n' => current.push('\n'),
                    'r' => current.push('\r'),
                    _ => return None,
                }
            }
            '\t' => {
                fields.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    Some(fields)
}
