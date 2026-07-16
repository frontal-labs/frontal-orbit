//! # Orbit Memory
//!
//! Semantic memory and lightweight knowledge graph primitives.

pub mod graph_neo4j_adapter;
pub mod persistent_metadata_store;
pub mod vector_pinecone_adapter;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub use graph_neo4j_adapter::{
    Neo4jConfig, Neo4jGraphStoreAdapter, Neo4jHttpTransportConfig, Neo4jTransport,
    ReqwestNeo4jTransport,
};
use orbit_embeddings::{cosine_similarity, EmbeddingProvider, LocalMlEmbeddingProvider};
pub use persistent_metadata_store::PersistentFileMetadataStore;
use serde::{Deserialize, Serialize};
pub use vector_pinecone_adapter::{
    PineconeConfig, PineconeHttpTransportConfig, PineconeTransport, PineconeVectorStoreAdapter,
    ReqwestPineconeTransport,
};

const ORBIT_MEMORY_METADATA_PATH: &str = "ORBIT_MEMORY_METADATA_PATH";
const ORBIT_MEMORY_PINECONE_URL: &str = "ORBIT_MEMORY_PINECONE_URL";
const ORBIT_MEMORY_PINECONE_NAMESPACE: &str = "ORBIT_MEMORY_PINECONE_NAMESPACE";
const ORBIT_MEMORY_PINECONE_API_KEY: &str = "ORBIT_MEMORY_PINECONE_API_KEY";
const ORBIT_MEMORY_NEO4J_URL: &str = "ORBIT_MEMORY_NEO4J_URL";
const ORBIT_MEMORY_NEO4J_DATABASE: &str = "ORBIT_MEMORY_NEO4J_DATABASE";
const ORBIT_MEMORY_NEO4J_USERNAME: &str = "ORBIT_MEMORY_NEO4J_USERNAME";
const ORBIT_MEMORY_NEO4J_PASSWORD: &str = "ORBIT_MEMORY_NEO4J_PASSWORD";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryScope {
    pub session_id: String,
    pub repo_id: Option<String>,
    pub branch_id: Option<String>,
}

impl MemoryScope {
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            repo_id: None,
            branch_id: None,
        }
    }

    #[must_use]
    pub fn with_repo_id(mut self, repo_id: impl Into<String>) -> Self {
        self.repo_id = Some(repo_id.into());
        self
    }

    #[must_use]
    pub fn with_branch_id(mut self, branch_id: impl Into<String>) -> Self {
        self.branch_id = Some(branch_id.into());
        self
    }
}

impl Default for MemoryScope {
    fn default() -> Self {
        Self::new("global")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub source: String,
    pub text: String,
    pub tags: Vec<String>,
    pub embedding: Vec<f32>,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySearchResult {
    pub id: String,
    pub source: String,
    pub text: String,
    pub score: f32,
    pub embedding_model: String,
    pub embedding_provider: String,
    pub embedding_dimension: usize,
    pub embedding_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KgEntity {
    pub id: String,
    pub label: String,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KgRelation {
    pub subject_id: String,
    pub predicate: String,
    pub object_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryMetadata {
    pub id: String,
    pub source: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at_ms: u128,
    pub embedding_model: String,
    pub embedding_provider: String,
    pub embedding_dimension: usize,
    pub embedding_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorSearchHit {
    pub id: String,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySearchRequest {
    pub query: String,
    pub top_k: usize,
    pub tags: Vec<String>,
    pub min_score: Option<f32>,
    pub source_filter: Option<String>,
    pub preferred_source: Option<String>,
    pub rerank_policy: MemoryRerankPolicy,
}

impl MemorySearchRequest {
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            top_k: 5,
            tags: Vec::new(),
            min_score: None,
            source_filter: None,
            preferred_source: None,
            rerank_policy: MemoryRerankPolicy::default(),
        }
    }

    #[must_use]
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    #[must_use]
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    #[must_use]
    pub fn with_source_filter(mut self, source_filter: impl Into<String>) -> Self {
        self.source_filter = Some(source_filter.into());
        self
    }

    #[must_use]
    pub fn with_preferred_source(mut self, preferred_source: impl Into<String>) -> Self {
        self.preferred_source = Some(preferred_source.into());
        self
    }

    #[must_use]
    pub fn with_rerank_policy(mut self, rerank_policy: MemoryRerankPolicy) -> Self {
        self.rerank_policy = rerank_policy;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySearchDiagnostics {
    pub candidate_count: usize,
    pub model_mismatch_drops: usize,
    pub source_filter_drops: usize,
    pub tag_filter_drops: usize,
    pub min_score_drops: usize,
    pub returned_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryRerankPolicy {
    pub vector_weight: f32,
    pub lexical_weight: f32,
    pub tag_weight: f32,
    pub source_weight: f32,
    pub recency_weight: f32,
    pub recency_half_life_ms: u128,
}

impl Default for MemoryRerankPolicy {
    fn default() -> Self {
        Self {
            vector_weight: 0.65,
            lexical_weight: 0.2,
            tag_weight: 0.08,
            source_weight: 0.04,
            recency_weight: 0.03,
            recency_half_life_ms: 1000 * 60 * 60 * 24 * 7,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryBackendConfig {
    pub metadata_path: Option<PathBuf>,
    pub pinecone_url: Option<String>,
    pub pinecone_namespace: Option<String>,
    pub pinecone_api_key: Option<String>,
    pub neo4j_url: Option<String>,
    pub neo4j_database: Option<String>,
    pub neo4j_username: Option<String>,
    pub neo4j_password: Option<String>,
}

impl MemoryBackendConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            metadata_path: env_var(ORBIT_MEMORY_METADATA_PATH).map(PathBuf::from),
            pinecone_url: env_var(ORBIT_MEMORY_PINECONE_URL),
            pinecone_namespace: env_var(ORBIT_MEMORY_PINECONE_NAMESPACE),
            pinecone_api_key: env_var(ORBIT_MEMORY_PINECONE_API_KEY),
            neo4j_url: env_var(ORBIT_MEMORY_NEO4J_URL),
            neo4j_database: env_var(ORBIT_MEMORY_NEO4J_DATABASE),
            neo4j_username: env_var(ORBIT_MEMORY_NEO4J_USERNAME),
            neo4j_password: env_var(ORBIT_MEMORY_NEO4J_PASSWORD),
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ScopedEntityKey {
    scope: MemoryScope,
    id: String,
}

impl ScopedEntityKey {
    fn new(scope: &MemoryScope, id: impl Into<String>) -> Self {
        Self {
            scope: scope.clone(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ScopedRelationRecord {
    scope: MemoryScope,
    subject_id: String,
    predicate: String,
    object_id: String,
}

impl ScopedRelationRecord {
    fn new(scope: &MemoryScope, relation: KgRelation) -> Self {
        Self {
            scope: scope.clone(),
            subject_id: relation.subject_id,
            predicate: relation.predicate,
            object_id: relation.object_id,
        }
    }

    fn to_relation(&self) -> KgRelation {
        KgRelation {
            subject_id: self.subject_id.clone(),
            predicate: self.predicate.clone(),
            object_id: self.object_id.clone(),
        }
    }
}

pub trait MemoryMetadataStore: Send + Sync {
    fn upsert_item(&self, scope: &MemoryScope, item: MemoryMetadata);

    fn get_item(&self, scope: &MemoryScope, id: &str) -> Option<MemoryMetadata>;

    fn list_items(&self, scope: &MemoryScope) -> Vec<MemoryMetadata>;

    fn count_items(&self, scope: &MemoryScope) -> usize;

    fn delete_item(&self, scope: &MemoryScope, id: &str) -> bool;
}

pub trait MemoryVectorStore: Send + Sync {
    fn upsert_embedding(&self, scope: &MemoryScope, id: &str, embedding: Vec<f32>);

    fn similarity_search(
        &self,
        scope: &MemoryScope,
        query_embedding: &[f32],
    ) -> Vec<VectorSearchHit>;

    fn delete_embedding(&self, scope: &MemoryScope, id: &str) -> bool;
}

pub trait MemoryGraphStore: Send + Sync {
    fn upsert_entity(&self, scope: &MemoryScope, entity: KgEntity);

    fn add_relation(&self, scope: &MemoryScope, relation: KgRelation);

    fn list_entities(&self, scope: &MemoryScope) -> Vec<KgEntity>;

    fn list_relations(&self, scope: &MemoryScope) -> Vec<KgRelation>;

    fn neighbors(&self, scope: &MemoryScope, entity_id: &str) -> Vec<KgRelation>;

    fn delete_entity(&self, scope: &MemoryScope, entity_id: &str) -> bool;

    fn delete_relation(&self, scope: &MemoryScope, relation: &KgRelation) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMetadataStore {
    inner: Arc<RwLock<BTreeMap<ScopedMemoryKey, MemoryMetadata>>>,
}

impl MemoryMetadataStore for InMemoryMetadataStore {
    fn upsert_item(&self, scope: &MemoryScope, item: MemoryMetadata) {
        let key = ScopedMemoryKey::new(scope, item.id.clone());
        let mut state = self.inner.write().expect("memory metadata lock poisoned");
        state.insert(key, item);
    }

    fn get_item(&self, scope: &MemoryScope, id: &str) -> Option<MemoryMetadata> {
        let key = ScopedMemoryKey::new(scope, id.to_string());
        let state = self.inner.read().expect("memory metadata lock poisoned");
        state.get(&key).cloned()
    }

    fn list_items(&self, scope: &MemoryScope) -> Vec<MemoryMetadata> {
        let state = self.inner.read().expect("memory metadata lock poisoned");
        state
            .iter()
            .filter(|(key, _)| key.scope == *scope)
            .map(|(_, value)| value.clone())
            .collect()
    }

    fn count_items(&self, scope: &MemoryScope) -> usize {
        let state = self.inner.read().expect("memory metadata lock poisoned");
        state.keys().filter(|key| key.scope == *scope).count()
    }

    fn delete_item(&self, scope: &MemoryScope, id: &str) -> bool {
        let key = ScopedMemoryKey::new(scope, id.to_string());
        let mut state = self.inner.write().expect("memory metadata lock poisoned");
        state.remove(&key).is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryVectorStore {
    inner: Arc<RwLock<BTreeMap<ScopedMemoryKey, Vec<f32>>>>,
}

impl MemoryVectorStore for InMemoryVectorStore {
    fn upsert_embedding(&self, scope: &MemoryScope, id: &str, embedding: Vec<f32>) {
        let key = ScopedMemoryKey::new(scope, id.to_string());
        let mut state = self.inner.write().expect("memory vector lock poisoned");
        state.insert(key, embedding);
    }

    fn similarity_search(
        &self,
        scope: &MemoryScope,
        query_embedding: &[f32],
    ) -> Vec<VectorSearchHit> {
        let state = self.inner.read().expect("memory vector lock poisoned");
        let mut hits = state
            .iter()
            .filter(|(key, _)| key.scope == *scope)
            .map(|(key, embedding)| VectorSearchHit {
                id: key.id.clone(),
                score: cosine_similarity(query_embedding, embedding),
            })
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| right.score.total_cmp(&left.score));
        hits
    }

    fn delete_embedding(&self, scope: &MemoryScope, id: &str) -> bool {
        let key = ScopedMemoryKey::new(scope, id.to_string());
        let mut state = self.inner.write().expect("memory vector lock poisoned");
        state.remove(&key).is_some()
    }
}

#[derive(Debug, Default)]
struct InMemoryGraphState {
    entities: BTreeMap<ScopedEntityKey, KgEntity>,
    relations: BTreeSet<ScopedRelationRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryGraphStore {
    inner: Arc<RwLock<InMemoryGraphState>>,
}

impl MemoryGraphStore for InMemoryGraphStore {
    fn upsert_entity(&self, scope: &MemoryScope, entity: KgEntity) {
        let key = ScopedEntityKey::new(scope, entity.id.clone());
        let mut state = self.inner.write().expect("memory graph lock poisoned");
        state.entities.insert(key, entity);
    }

    fn add_relation(&self, scope: &MemoryScope, relation: KgRelation) {
        let mut state = self.inner.write().expect("memory graph lock poisoned");
        state
            .relations
            .insert(ScopedRelationRecord::new(scope, relation));
    }

    fn list_entities(&self, scope: &MemoryScope) -> Vec<KgEntity> {
        let state = self.inner.read().expect("memory graph lock poisoned");
        state
            .entities
            .iter()
            .filter(|(key, _)| key.scope == *scope)
            .map(|(_, entity)| entity.clone())
            .collect()
    }

    fn list_relations(&self, scope: &MemoryScope) -> Vec<KgRelation> {
        let state = self.inner.read().expect("memory graph lock poisoned");
        state
            .relations
            .iter()
            .filter(|relation| relation.scope == *scope)
            .map(ScopedRelationRecord::to_relation)
            .collect()
    }

    fn neighbors(&self, scope: &MemoryScope, entity_id: &str) -> Vec<KgRelation> {
        let state = self.inner.read().expect("memory graph lock poisoned");
        state
            .relations
            .iter()
            .filter(|relation| relation.scope == *scope)
            .filter(|relation| relation.subject_id == entity_id || relation.object_id == entity_id)
            .map(ScopedRelationRecord::to_relation)
            .collect()
    }

    fn delete_entity(&self, scope: &MemoryScope, entity_id: &str) -> bool {
        let key = ScopedEntityKey::new(scope, entity_id.to_string());
        let mut state = self.inner.write().expect("memory graph lock poisoned");
        let removed_entity = state.entities.remove(&key).is_some();
        let original_len = state.relations.len();
        state.relations.retain(|relation| {
            !(relation.scope == *scope
                && (relation.subject_id == entity_id || relation.object_id == entity_id))
        });
        removed_entity || state.relations.len() != original_len
    }

    fn delete_relation(&self, scope: &MemoryScope, relation: &KgRelation) -> bool {
        let mut state = self.inner.write().expect("memory graph lock poisoned");
        state
            .relations
            .remove(&ScopedRelationRecord::new(scope, relation.clone()))
    }
}

#[derive(Clone)]
pub struct MemoryService {
    provider: Arc<dyn EmbeddingProvider>,
    metadata_store: Arc<dyn MemoryMetadataStore>,
    vector_store: Arc<dyn MemoryVectorStore>,
    graph_store: Arc<dyn MemoryGraphStore>,
    backend_config: MemoryBackendConfig,
}

#[derive(Clone)]
#[derive(Default)]
pub struct SemanticMemoryEngine {
    service: MemoryService,
    default_scope: MemoryScope,
}

impl MemoryService {
    #[must_use]
    pub fn with_backends(
        provider: Arc<dyn EmbeddingProvider>,
        metadata_store: Arc<dyn MemoryMetadataStore>,
        vector_store: Arc<dyn MemoryVectorStore>,
        graph_store: Arc<dyn MemoryGraphStore>,
    ) -> Self {
        Self::with_backends_and_config(
            provider,
            metadata_store,
            vector_store,
            graph_store,
            MemoryBackendConfig::default(),
        )
    }

    #[must_use]
    fn with_backends_and_config(
        provider: Arc<dyn EmbeddingProvider>,
        metadata_store: Arc<dyn MemoryMetadataStore>,
        vector_store: Arc<dyn MemoryVectorStore>,
        graph_store: Arc<dyn MemoryGraphStore>,
        backend_config: MemoryBackendConfig,
    ) -> Self {
        Self {
            provider,
            metadata_store,
            vector_store,
            graph_store,
            backend_config,
        }
    }

    #[must_use]
    pub fn with_provider(provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self::with_backends(
            provider,
            Arc::new(InMemoryMetadataStore::default()),
            Arc::new(InMemoryVectorStore::default()),
            Arc::new(InMemoryGraphStore::default()),
        )
    }

    #[must_use]
    pub fn with_persistent_metadata_path(path: impl Into<PathBuf>) -> Self {
        let config = MemoryBackendConfig {
            metadata_path: Some(path.into()),
            ..MemoryBackendConfig::default()
        };
        Self::from_backend_config(config)
    }

    #[must_use]
    pub fn from_backend_config(config: MemoryBackendConfig) -> Self {
        Self::from_backend_config_with_provider(
            Arc::new(LocalMlEmbeddingProvider::default()),
            config,
        )
    }

    #[must_use]
    pub fn from_backend_config_with_provider(
        provider: Arc<dyn EmbeddingProvider>,
        config: MemoryBackendConfig,
    ) -> Self {
        let metadata_store: Arc<dyn MemoryMetadataStore> = match config.metadata_path.clone() {
            Some(path) => Arc::new(PersistentFileMetadataStore::new(path)),
            None => Arc::new(InMemoryMetadataStore::default()),
        };

        let vector_store: Arc<dyn MemoryVectorStore> = match config.pinecone_url.clone() {
            Some(base_url) => {
                let mut transport_config = PineconeHttpTransportConfig::new(base_url);
                if let Some(api_key) = config.pinecone_api_key.clone() {
                    transport_config = transport_config.with_api_key(api_key);
                }
                Arc::new(PineconeVectorStoreAdapter::new(
                    PineconeConfig::new(config.pinecone_namespace.clone()),
                    ReqwestPineconeTransport::new(transport_config),
                ))
            }
            None => Arc::new(InMemoryVectorStore::default()),
        };

        let graph_store: Arc<dyn MemoryGraphStore> =
            match (config.neo4j_url.clone(), config.neo4j_database.clone()) {
                (Some(base_url), Some(database)) => {
                    let mut transport_config = Neo4jHttpTransportConfig::new(base_url);
                    if let (Some(username), Some(password)) =
                        (config.neo4j_username.clone(), config.neo4j_password.clone())
                    {
                        transport_config = transport_config.with_basic_auth(username, password);
                    }
                    Arc::new(Neo4jGraphStoreAdapter::new(
                        Neo4jConfig::new(database),
                        ReqwestNeo4jTransport::new(transport_config),
                    ))
                }
                _ => Arc::new(InMemoryGraphStore::default()),
            };

        Self::with_backends_and_config(provider, metadata_store, vector_store, graph_store, config)
    }

    #[must_use]
    pub fn from_env() -> Self {
        Self::from_backend_config(MemoryBackendConfig::from_env())
    }

    pub fn upsert_memory(
        &self,
        scope: &MemoryScope,
        id: impl Into<String>,
        source: impl Into<String>,
        text: impl Into<String>,
        tags: Vec<String>,
    ) {
        let id = id.into();
        let text = text.into();
        let model_info = self.provider.model_info();
        let embedding = self.provider.embed(&text);
        let item = MemoryMetadata {
            id: id.clone(),
            source: source.into(),
            text,
            tags,
            created_at_ms: now_ms(),
            embedding_model: model_info.model_name,
            embedding_provider: model_info.provider,
            embedding_dimension: model_info.dimension,
            embedding_revision: model_info.revision,
        };

        self.metadata_store.upsert_item(scope, item);
        self.vector_store.upsert_embedding(scope, &id, embedding);
    }

    #[must_use] 
    pub fn delete_memory(&self, scope: &MemoryScope, id: &str) -> bool {
        let removed_metadata = self.metadata_store.delete_item(scope, id);
        let removed_vector = self.vector_store.delete_embedding(scope, id);
        removed_metadata || removed_vector
    }

    #[must_use]
    pub fn search(
        &self,
        scope: &MemoryScope,
        query: &str,
        top_k: usize,
        tags: Option<&[String]>,
    ) -> Vec<MemorySearchResult> {
        self.search_with_request(
            scope,
            &MemorySearchRequest {
                query: query.to_string(),
                top_k,
                tags: tags.map_or_else(Vec::new, <[std::string::String]>::to_vec),
                min_score: None,
                source_filter: None,
                preferred_source: None,
                rerank_policy: MemoryRerankPolicy::default(),
            },
        )
    }

    #[must_use]
    pub fn search_with_request(
        &self,
        scope: &MemoryScope,
        request: &MemorySearchRequest,
    ) -> Vec<MemorySearchResult> {
        self.search_with_request_and_diagnostics(scope, request).0
    }

    #[must_use]
    pub fn search_with_request_and_diagnostics(
        &self,
        scope: &MemoryScope,
        request: &MemorySearchRequest,
    ) -> (Vec<MemorySearchResult>, MemorySearchDiagnostics) {
        if request.top_k == 0 {
            return (Vec::new(), MemorySearchDiagnostics::default());
        }

        let min_score = request.min_score.unwrap_or(f32::NEG_INFINITY);
        let model_info = self.provider.model_info();
        let query_terms = normalize_terms(&request.query);
        let query_embedding = self.provider.embed(&request.query);
        let filter_tags = request.tags.iter().cloned().collect::<BTreeSet<_>>();
        let source_filter = request.source_filter.as_deref().map(normalize_source);
        let preferred_source = request.preferred_source.as_deref().map(normalize_source);
        let now_ms = now_ms();
        let hits = self.vector_store.similarity_search(scope, &query_embedding);
        let mut diagnostics = MemorySearchDiagnostics {
            candidate_count: hits.len(),
            ..MemorySearchDiagnostics::default()
        };
        let mut scored = hits
            .into_iter()
            .filter_map(|hit| {
                let item = self.metadata_store.get_item(scope, &hit.id)?;
                if !embedding_metadata_matches(&item, &model_info) {
                    diagnostics.model_mismatch_drops += 1;
                    return None;
                }
                if let Some(source_filter) = source_filter.as_deref() {
                    if normalize_source(&item.source) != source_filter {
                        diagnostics.source_filter_drops += 1;
                        return None;
                    }
                }
                if !filter_tags.is_empty() && !item.tags.iter().any(|tag| filter_tags.contains(tag))
                {
                    diagnostics.tag_filter_drops += 1;
                    return None;
                }

                let lexical = lexical_overlap_score(&request.query, &item.text);
                let tag_score = tag_overlap_score(&filter_tags, &item.tags, &query_terms);
                let source_score = preferred_source.as_deref().map_or(0.0, |preferred| {
                    if normalize_source(&item.source) == preferred {
                        1.0
                    } else {
                        0.0
                    }
                });
                let recency_score = recency_boost(
                    item.created_at_ms,
                    now_ms,
                    request.rerank_policy.recency_half_life_ms,
                );
                let weighted_score = (hit.score * request.rerank_policy.vector_weight)
                    + (lexical * request.rerank_policy.lexical_weight)
                    + (tag_score * request.rerank_policy.tag_weight)
                    + (source_score * request.rerank_policy.source_weight)
                    + (recency_score * request.rerank_policy.recency_weight);
                if weighted_score < min_score {
                    diagnostics.min_score_drops += 1;
                    return None;
                }
                Some(MemorySearchResult {
                    id: item.id,
                    source: item.source,
                    text: item.text,
                    score: weighted_score,
                    embedding_model: item.embedding_model,
                    embedding_provider: item.embedding_provider,
                    embedding_dimension: item.embedding_dimension,
                    embedding_revision: item.embedding_revision,
                })
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| right.score.total_cmp(&left.score));
        scored.truncate(request.top_k);
        diagnostics.returned_count = scored.len();
        (scored, diagnostics)
    }

    pub fn upsert_entity(&self, scope: &MemoryScope, entity: KgEntity) {
        self.graph_store.upsert_entity(scope, entity);
    }

    pub fn add_relation(&self, scope: &MemoryScope, relation: KgRelation) {
        self.graph_store.add_relation(scope, relation);
    }

    #[must_use]
    pub fn list_entities(&self, scope: &MemoryScope) -> Vec<KgEntity> {
        self.graph_store.list_entities(scope)
    }

    #[must_use]
    pub fn list_relations(&self, scope: &MemoryScope) -> Vec<KgRelation> {
        self.graph_store.list_relations(scope)
    }

    #[must_use]
    pub fn neighbors(&self, scope: &MemoryScope, entity_id: &str) -> Vec<KgRelation> {
        self.graph_store.neighbors(scope, entity_id)
    }

    #[must_use] 
    pub fn delete_entity(&self, scope: &MemoryScope, entity_id: &str) -> bool {
        self.graph_store.delete_entity(scope, entity_id)
    }

    #[must_use] 
    pub fn delete_relation(&self, scope: &MemoryScope, relation: &KgRelation) -> bool {
        self.graph_store.delete_relation(scope, relation)
    }

    #[must_use]
    pub fn item_count(&self, scope: &MemoryScope) -> usize {
        self.metadata_store.count_items(scope)
    }

    #[must_use]
    pub fn list_memories(&self, scope: &MemoryScope) -> Vec<MemoryMetadata> {
        self.metadata_store.list_items(scope)
    }

    #[must_use]
    pub fn backend_config(&self) -> &MemoryBackendConfig {
        &self.backend_config
    }
}

impl Default for MemoryService {
    fn default() -> Self {
        Self::from_env()
    }
}


impl SemanticMemoryEngine {
    #[must_use]
    pub fn with_provider(provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            service: MemoryService::with_provider(provider),
            default_scope: MemoryScope::default(),
        }
    }

    #[must_use]
    pub fn with_persistent_metadata_path(path: impl Into<PathBuf>) -> Self {
        Self {
            service: MemoryService::with_persistent_metadata_path(path),
            default_scope: MemoryScope::default(),
        }
    }

    #[must_use]
    pub fn from_env() -> Self {
        Self {
            service: MemoryService::from_env(),
            default_scope: MemoryScope::default(),
        }
    }

    #[must_use]
    pub fn with_default_scope(mut self, scope: MemoryScope) -> Self {
        self.default_scope = scope;
        self
    }

    #[must_use]
    pub fn service(&self) -> &MemoryService {
        &self.service
    }

    pub fn upsert_memory_scoped(
        &self,
        scope: &MemoryScope,
        id: impl Into<String>,
        source: impl Into<String>,
        text: impl Into<String>,
        tags: Vec<String>,
    ) {
        self.service.upsert_memory(scope, id, source, text, tags);
    }

    #[must_use] 
    pub fn delete_memory_scoped(&self, scope: &MemoryScope, id: &str) -> bool {
        self.service.delete_memory(scope, id)
    }

    #[must_use]
    pub fn search_scoped(
        &self,
        scope: &MemoryScope,
        query: &str,
        top_k: usize,
        tags: Option<&[String]>,
    ) -> Vec<MemorySearchResult> {
        self.service.search(scope, query, top_k, tags)
    }

    #[must_use]
    pub fn search_scoped_with_request(
        &self,
        scope: &MemoryScope,
        request: &MemorySearchRequest,
    ) -> Vec<MemorySearchResult> {
        self.service.search_with_request(scope, request)
    }

    pub fn upsert_entity_scoped(&self, scope: &MemoryScope, entity: KgEntity) {
        self.service.upsert_entity(scope, entity);
    }

    pub fn add_relation_scoped(&self, scope: &MemoryScope, relation: KgRelation) {
        self.service.add_relation(scope, relation);
    }

    #[must_use]
    pub fn list_entities_scoped(&self, scope: &MemoryScope) -> Vec<KgEntity> {
        self.service.list_entities(scope)
    }

    #[must_use]
    pub fn list_relations_scoped(&self, scope: &MemoryScope) -> Vec<KgRelation> {
        self.service.list_relations(scope)
    }

    #[must_use]
    pub fn neighbors_scoped(&self, scope: &MemoryScope, entity_id: &str) -> Vec<KgRelation> {
        self.service.neighbors(scope, entity_id)
    }

    #[must_use] 
    pub fn delete_entity_scoped(&self, scope: &MemoryScope, entity_id: &str) -> bool {
        self.service.delete_entity(scope, entity_id)
    }

    #[must_use] 
    pub fn delete_relation_scoped(&self, scope: &MemoryScope, relation: &KgRelation) -> bool {
        self.service.delete_relation(scope, relation)
    }

    pub fn upsert_memory(
        &self,
        id: impl Into<String>,
        source: impl Into<String>,
        text: impl Into<String>,
        tags: Vec<String>,
    ) {
        self.upsert_memory_scoped(&self.default_scope, id, source, text, tags);
    }

    #[must_use] 
    pub fn delete_memory(&self, id: &str) -> bool {
        self.delete_memory_scoped(&self.default_scope, id)
    }

    #[must_use]
    pub fn search(
        &self,
        query: &str,
        top_k: usize,
        tags: Option<&[String]>,
    ) -> Vec<MemorySearchResult> {
        self.search_scoped(&self.default_scope, query, top_k, tags)
    }

    #[must_use]
    pub fn search_with_request(&self, request: &MemorySearchRequest) -> Vec<MemorySearchResult> {
        self.search_scoped_with_request(&self.default_scope, request)
    }

    pub fn upsert_entity(&self, entity: KgEntity) {
        self.upsert_entity_scoped(&self.default_scope, entity);
    }

    pub fn add_relation(&self, relation: KgRelation) {
        self.add_relation_scoped(&self.default_scope, relation);
    }

    #[must_use]
    pub fn list_entities(&self) -> Vec<KgEntity> {
        self.list_entities_scoped(&self.default_scope)
    }

    #[must_use]
    pub fn list_relations(&self) -> Vec<KgRelation> {
        self.list_relations_scoped(&self.default_scope)
    }

    #[must_use]
    pub fn neighbors(&self, entity_id: &str) -> Vec<KgRelation> {
        self.neighbors_scoped(&self.default_scope, entity_id)
    }

    #[must_use] 
    pub fn delete_entity(&self, entity_id: &str) -> bool {
        self.delete_entity_scoped(&self.default_scope, entity_id)
    }

    #[must_use] 
    pub fn delete_relation(&self, relation: &KgRelation) -> bool {
        self.delete_relation_scoped(&self.default_scope, relation)
    }

    #[must_use]
    pub fn item_count(&self) -> usize {
        self.item_count_scoped(&self.default_scope)
    }

    #[must_use]
    pub fn item_count_scoped(&self, scope: &MemoryScope) -> usize {
        self.service.item_count(scope)
    }

    #[must_use]
    pub fn list_memories_scoped(&self, scope: &MemoryScope) -> Vec<MemoryMetadata> {
        self.service.list_memories(scope)
    }
}

#[allow(clippy::cast_precision_loss)]
fn lexical_overlap_score(query: &str, candidate: &str) -> f32 {
    let q = normalize_terms(query);
    let c = normalize_terms(candidate);
    if q.is_empty() || c.is_empty() {
        return 0.0;
    }

    let overlap = q.intersection(&c).count() as f32;
    overlap / q.len() as f32
}

#[allow(clippy::cast_precision_loss)]
fn tag_overlap_score(
    filter_tags: &BTreeSet<String>,
    item_tags: &[String],
    query_terms: &BTreeSet<String>,
) -> f32 {
    if !filter_tags.is_empty() {
        let overlap = item_tags
            .iter()
            .filter(|tag| filter_tags.contains(*tag))
            .count() as f32;
        return overlap / filter_tags.len() as f32;
    }

    let item_tag_terms = item_tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    if item_tag_terms.is_empty() || query_terms.is_empty() {
        return 0.0;
    }
    let overlap = item_tag_terms.intersection(query_terms).count() as f32;
    overlap / item_tag_terms.len() as f32
}

fn normalize_terms(input: &str) -> BTreeSet<String> {
    input
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn env_var(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn embedding_metadata_matches(
    item: &MemoryMetadata,
    model_info: &orbit_embeddings::EmbeddingModelInfo,
) -> bool {
    item.embedding_model == model_info.model_name
        && item.embedding_provider == model_info.provider
        && item.embedding_dimension == model_info.dimension
        && item.embedding_revision == model_info.revision
}

fn normalize_source(source: &str) -> String {
    source.trim().to_ascii_lowercase()
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn recency_boost(created_at_ms: u128, now_ms: u128, half_life_ms: u128) -> f32 {
    if half_life_ms == 0 {
        return 0.0;
    }
    let age = now_ms.saturating_sub(created_at_ms) as f64;
    let half_life = half_life_ms as f64;
    (1.0 / (1.0 + (age / half_life))) as f32
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryGraphStore, InMemoryMetadataStore, InMemoryVectorStore, KgEntity, KgRelation,
        MemoryBackendConfig, MemoryMetadata, MemoryMetadataStore, MemoryRerankPolicy, MemoryScope,
        MemorySearchRequest, MemoryService, MemoryVectorStore, SemanticMemoryEngine,
    };
    use orbit_embeddings::{EmbeddingModelConfig, EmbeddingProvider, LocalMlEmbeddingProvider};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn semantic_search_returns_relevant_hits() {
        let memory = SemanticMemoryEngine::default();
        memory.upsert_memory(
            "mem-1",
            "git",
            "Prefer concise commit messages with imperative verbs",
            vec!["style".to_string(), "git".to_string()],
        );
        memory.upsert_memory(
            "mem-2",
            "infra",
            "Use docker compose for local postgres and pinecone",
            vec!["ops".to_string()],
        );

        let results = memory.search("commit message style", 2, None);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "mem-1");
        assert_eq!(results[0].embedding_model, "local-hash-embedding-v1");
        assert_eq!(results[0].embedding_provider, "local-hash");
        assert_eq!(results[0].embedding_dimension, 384);
    }

    #[test]
    fn knowledge_graph_neighbors_are_bidirectional() {
        let memory = SemanticMemoryEngine::default();
        memory.upsert_entity(KgEntity {
            id: "crate:tools".to_string(),
            label: "orbit-tools".to_string(),
            entity_type: "crate".to_string(),
        });
        memory.upsert_entity(KgEntity {
            id: "crate:runtime".to_string(),
            label: "orbit-runtime".to_string(),
            entity_type: "crate".to_string(),
        });
        memory.add_relation(KgRelation {
            subject_id: "crate:tools".to_string(),
            predicate: "depends_on".to_string(),
            object_id: "crate:runtime".to_string(),
        });

        let neighbors = memory.neighbors("crate:runtime");
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].predicate, "depends_on");
    }

    #[test]
    fn legacy_default_scope_behavior_is_preserved() {
        let default_scope = MemoryScope::new("legacy-session");
        let other_scope = MemoryScope::new("other-session");
        let memory = SemanticMemoryEngine::default().with_default_scope(default_scope.clone());

        memory.upsert_memory(
            "legacy-1",
            "legacy",
            "default-scoped memory entry",
            vec!["legacy".to_string()],
        );

        assert_eq!(memory.item_count(), 1);
        assert_eq!(memory.item_count_scoped(&default_scope), 1);
        assert_eq!(memory.item_count_scoped(&other_scope), 0);
    }

    #[test]
    fn scoped_memory_search_is_isolated() {
        let memory = SemanticMemoryEngine::default();
        let scope_a = MemoryScope::new("session-a");
        let scope_b = MemoryScope::new("session-b");
        memory.upsert_memory_scoped(
            &scope_a,
            "a-1",
            "scope-a",
            "use alpha_graph token for scope a",
            vec!["a".to_string()],
        );
        memory.upsert_memory_scoped(
            &scope_b,
            "b-1",
            "scope-b",
            "use beta_cluster token for scope b",
            vec!["b".to_string()],
        );

        let results_a = memory.search_scoped(&scope_a, "alpha_graph", 10, None);
        let results_b = memory.search_scoped(&scope_b, "alpha_graph", 10, None);
        assert_eq!(results_a.len(), 1);
        assert_eq!(results_a[0].id, "a-1");
        assert_eq!(results_b.len(), 1);
        assert_eq!(results_b[0].id, "b-1");
        assert_eq!(memory.item_count_scoped(&scope_a), 1);
        assert_eq!(memory.item_count_scoped(&scope_b), 1);
    }

    #[test]
    fn scoped_kg_neighbors_are_isolated() {
        let memory = SemanticMemoryEngine::default();
        let scope_a = MemoryScope::new("session-a");
        let scope_b = MemoryScope::new("session-b");
        memory.upsert_entity_scoped(
            &scope_a,
            KgEntity {
                id: "crate:tools".to_string(),
                label: "orbit-tools".to_string(),
                entity_type: "crate".to_string(),
            },
        );
        memory.upsert_entity_scoped(
            &scope_a,
            KgEntity {
                id: "crate:runtime".to_string(),
                label: "orbit-runtime".to_string(),
                entity_type: "crate".to_string(),
            },
        );
        memory.add_relation_scoped(
            &scope_a,
            KgRelation {
                subject_id: "crate:tools".to_string(),
                predicate: "depends_on".to_string(),
                object_id: "crate:runtime".to_string(),
            },
        );

        memory.upsert_entity_scoped(
            &scope_b,
            KgEntity {
                id: "crate:tools".to_string(),
                label: "orbit-tools".to_string(),
                entity_type: "crate".to_string(),
            },
        );
        memory.upsert_entity_scoped(
            &scope_b,
            KgEntity {
                id: "crate:runtime".to_string(),
                label: "orbit-runtime".to_string(),
                entity_type: "crate".to_string(),
            },
        );

        let neighbors_a = memory.neighbors_scoped(&scope_a, "crate:runtime");
        let neighbors_b = memory.neighbors_scoped(&scope_b, "crate:runtime");
        assert_eq!(neighbors_a.len(), 1);
        assert_eq!(neighbors_b.len(), 0);
    }

    #[test]
    fn min_score_can_filter_irrelevant_results() {
        let memory = SemanticMemoryEngine::default();
        memory.upsert_memory(
            "mem-1",
            "git",
            "Prefer concise commit messages with imperative verbs",
            vec!["style".to_string()],
        );

        let results = memory.search_with_request(
            &MemorySearchRequest::new("totally unrelated galaxy query")
                .with_top_k(5)
                .with_min_score(0.95),
        );
        assert!(results.is_empty());
    }

    #[test]
    fn delete_memory_removes_items_from_search_and_count() {
        let memory = SemanticMemoryEngine::default();
        memory.upsert_memory(
            "mem-1",
            "git",
            "Prefer concise commit messages with imperative verbs",
            vec!["style".to_string()],
        );
        assert_eq!(memory.item_count(), 1);

        assert!(memory.delete_memory("mem-1"));
        assert_eq!(memory.item_count(), 0);
        assert!(memory.search("commit message style", 2, None).is_empty());
        assert!(!memory.delete_memory("missing"));
    }

    #[test]
    fn search_ignores_memories_with_stale_embedding_metadata() {
        let metadata_store = Arc::new(InMemoryMetadataStore::default());
        let vector_store = Arc::new(InMemoryVectorStore::default());
        let graph_store = Arc::new(InMemoryGraphStore::default());
        let scope = MemoryScope::new("session-a");

        let writer = MemoryService::with_backends(
            Arc::new(LocalMlEmbeddingProvider::default()),
            metadata_store.clone(),
            vector_store.clone(),
            graph_store.clone(),
        );
        writer.upsert_memory(
            &scope,
            "mem-1",
            "git",
            "Prefer concise commit messages with imperative verbs",
            vec!["style".to_string()],
        );

        let reader = MemoryService::with_backends(
            Arc::new(LocalMlEmbeddingProvider::new(EmbeddingModelConfig {
                model_name: "local-hash-embedding-v2".to_string(),
                ..EmbeddingModelConfig::default()
            })),
            metadata_store,
            vector_store,
            graph_store,
        );

        let results = reader.search(&scope, "commit message style", 5, None);
        assert!(results.is_empty());
    }

    #[test]
    fn delete_entity_removes_neighbor_relations() {
        let memory = SemanticMemoryEngine::default();
        memory.upsert_entity(KgEntity {
            id: "crate:tools".to_string(),
            label: "orbit-tools".to_string(),
            entity_type: "crate".to_string(),
        });
        memory.upsert_entity(KgEntity {
            id: "crate:runtime".to_string(),
            label: "orbit-runtime".to_string(),
            entity_type: "crate".to_string(),
        });
        memory.add_relation(KgRelation {
            subject_id: "crate:tools".to_string(),
            predicate: "depends_on".to_string(),
            object_id: "crate:runtime".to_string(),
        });

        assert!(memory.delete_entity("crate:runtime"));
        assert!(memory.neighbors("crate:runtime").is_empty());
        assert!(!memory.delete_entity("missing"));
    }

    #[test]
    fn delete_relation_removes_single_edge() {
        let memory = SemanticMemoryEngine::default();
        let relation = KgRelation {
            subject_id: "crate:tools".to_string(),
            predicate: "depends_on".to_string(),
            object_id: "crate:runtime".to_string(),
        };
        memory.add_relation(relation.clone());

        assert!(memory.delete_relation(&relation));
        assert!(memory.neighbors("crate:runtime").is_empty());
        assert!(!memory.delete_relation(&relation));
    }

    #[test]
    fn rerank_policy_can_prefer_matching_source() {
        let metadata_store = Arc::new(InMemoryMetadataStore::default());
        let vector_store = Arc::new(InMemoryVectorStore::default());
        let graph_store = Arc::new(InMemoryGraphStore::default());
        let provider = Arc::new(LocalMlEmbeddingProvider::default());
        let model_info = provider.model_info();
        let scope = MemoryScope::new("session-a");
        let shared_embedding = provider.embed("shared search text");

        metadata_store.upsert_item(
            &scope,
            MemoryMetadata {
                id: "mem-git".to_string(),
                source: "git:commit".to_string(),
                text: "shared search text".to_string(),
                tags: vec![],
                created_at_ms: 100,
                embedding_model: model_info.model_name.clone(),
                embedding_provider: model_info.provider.clone(),
                embedding_dimension: model_info.dimension,
                embedding_revision: model_info.revision.clone(),
            },
        );
        vector_store.upsert_embedding(&scope, "mem-git", shared_embedding.clone());
        metadata_store.upsert_item(
            &scope,
            MemoryMetadata {
                id: "mem-docs".to_string(),
                source: "docs".to_string(),
                text: "shared search text".to_string(),
                tags: vec![],
                created_at_ms: 100,
                embedding_model: model_info.model_name.clone(),
                embedding_provider: model_info.provider.clone(),
                embedding_dimension: model_info.dimension,
                embedding_revision: model_info.revision.clone(),
            },
        );
        vector_store.upsert_embedding(&scope, "mem-docs", shared_embedding);

        let service =
            MemoryService::with_backends(provider, metadata_store, vector_store, graph_store);
        let results = service.search_with_request(
            &scope,
            &MemorySearchRequest::new("shared search text")
                .with_preferred_source("git:commit")
                .with_rerank_policy(MemoryRerankPolicy {
                    vector_weight: 0.5,
                    lexical_weight: 0.2,
                    tag_weight: 0.0,
                    source_weight: 0.3,
                    recency_weight: 0.0,
                    recency_half_life_ms: 1,
                }),
        );

        assert_eq!(results[0].id, "mem-git");
    }

    #[test]
    fn rerank_policy_can_prefer_recent_entries() {
        let metadata_store = Arc::new(InMemoryMetadataStore::default());
        let vector_store = Arc::new(InMemoryVectorStore::default());
        let graph_store = Arc::new(InMemoryGraphStore::default());
        let provider = Arc::new(LocalMlEmbeddingProvider::default());
        let model_info = provider.model_info();
        let scope = MemoryScope::new("session-a");
        let shared_embedding = provider.embed("shared search text");
        let now = super::now_ms();

        metadata_store.upsert_item(
            &scope,
            MemoryMetadata {
                id: "mem-new".to_string(),
                source: "git:commit".to_string(),
                text: "shared search text".to_string(),
                tags: vec![],
                created_at_ms: now,
                embedding_model: model_info.model_name.clone(),
                embedding_provider: model_info.provider.clone(),
                embedding_dimension: model_info.dimension,
                embedding_revision: model_info.revision.clone(),
            },
        );
        vector_store.upsert_embedding(&scope, "mem-new", shared_embedding.clone());
        metadata_store.upsert_item(
            &scope,
            MemoryMetadata {
                id: "mem-old".to_string(),
                source: "git:commit".to_string(),
                text: "shared search text".to_string(),
                tags: vec![],
                created_at_ms: now.saturating_sub(1000 * 60 * 60 * 24 * 30),
                embedding_model: model_info.model_name.clone(),
                embedding_provider: model_info.provider.clone(),
                embedding_dimension: model_info.dimension,
                embedding_revision: model_info.revision.clone(),
            },
        );
        vector_store.upsert_embedding(&scope, "mem-old", shared_embedding);

        let service =
            MemoryService::with_backends(provider, metadata_store, vector_store, graph_store);
        let results = service.search_with_request(
            &scope,
            &MemorySearchRequest::new("shared search text").with_rerank_policy(
                MemoryRerankPolicy {
                    vector_weight: 0.4,
                    lexical_weight: 0.1,
                    tag_weight: 0.0,
                    source_weight: 0.0,
                    recency_weight: 0.5,
                    recency_half_life_ms: 1000 * 60 * 60 * 24,
                },
            ),
        );

        assert_eq!(results[0].id, "mem-new");
    }

    #[test]
    fn backend_config_persists_metadata_when_path_is_configured() {
        let path = std::env::temp_dir().join(format!(
            "orbit-memory-backend-config-{}.tsv",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let scope = MemoryScope::new("session-a");
        let config = MemoryBackendConfig {
            metadata_path: Some(path.clone()),
            ..MemoryBackendConfig::default()
        };

        let service = MemoryService::from_backend_config(config.clone());
        service.upsert_memory(
            &scope,
            "persisted-1",
            "test",
            "persist this metadata entry",
            vec!["persisted".to_string()],
        );
        assert_eq!(service.item_count(&scope), 1);

        let reloaded = MemoryService::from_backend_config(config);
        assert_eq!(reloaded.item_count(&scope), 1);

        let _ = std::fs::remove_file(path);
    }
}
