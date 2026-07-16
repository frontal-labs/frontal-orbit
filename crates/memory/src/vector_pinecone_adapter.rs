//! Pinecone-backed HTTP adapter for `MemoryVectorStore`.
//!
//! This module stays decoupled from any concrete HTTP client crate.

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::{MemoryScope, MemoryVectorStore, VectorSearchHit};

const PINECONE_API_VERSION: &str = "2025-10";

/// Minimal transport boundary for Pinecone HTTP calls.
pub trait PineconeTransport: Send + Sync {
    fn post(&self, path: &str, body: &str) -> Result<String, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PineconeConfig {
    pub namespace_prefix: Option<String>,
}

impl PineconeConfig {
    #[must_use]
    pub fn new(namespace_prefix: Option<String>) -> Self {
        Self { namespace_prefix }
    }

    fn namespace_for_scope(&self, scope: &MemoryScope) -> String {
        let scope_namespace = scoped_namespace(scope);
        match self
            .namespace_prefix
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            Some(prefix) => format!("{prefix}:{scope_namespace}"),
            None => scope_namespace,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PineconeHttpTransportConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl PineconeHttpTransportConfig {
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
        }
    }

    #[must_use]
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct ReqwestPineconeTransport {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl ReqwestPineconeTransport {
    #[must_use]
    pub fn new(config: PineconeHttpTransportConfig) -> Self {
        Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key: config.api_key,
            client: Client::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeVectorMetadata {
    pub session_id: String,
    pub repo_id: Option<String>,
    pub branch_id: Option<String>,
    pub memory_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeVector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: PineconeVectorMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeUpsertRequest {
    pub vectors: Vec<PineconeVector>,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeDeleteRequest {
    pub ids: Vec<String>,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeQueryRequest {
    pub vector: Vec<f32>,
    #[serde(rename = "topK")]
    pub top_k: usize,
    pub namespace: String,
    #[serde(rename = "includeMetadata")]
    pub include_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeQueryMatch {
    pub id: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PineconeQueryResponse {
    pub matches: Vec<PineconeQueryMatch>,
}

#[derive(Debug)]
pub struct PineconeVectorStoreAdapter<T: PineconeTransport> {
    config: PineconeConfig,
    transport: T,
}

impl<T: PineconeTransport> PineconeVectorStoreAdapter<T> {
    #[must_use]
    pub fn new(config: PineconeConfig, transport: T) -> Self {
        Self { config, transport }
    }

    #[must_use]
    pub fn build_upsert_request(
        &self,
        scope: &MemoryScope,
        id: &str,
        embedding: Vec<f32>,
    ) -> PineconeUpsertRequest {
        PineconeUpsertRequest {
            vectors: vec![PineconeVector {
                id: scoped_vector_id(scope, id),
                values: embedding,
                metadata: PineconeVectorMetadata {
                    session_id: scope.session_id.clone(),
                    repo_id: scope.repo_id.clone(),
                    branch_id: scope.branch_id.clone(),
                    memory_id: id.to_string(),
                },
            }],
            namespace: self.config.namespace_for_scope(scope),
        }
    }

    #[must_use]
    pub fn build_query_request(
        &self,
        scope: &MemoryScope,
        query_embedding: &[f32],
        top_k: usize,
    ) -> PineconeQueryRequest {
        PineconeQueryRequest {
            vector: query_embedding.to_vec(),
            top_k,
            namespace: self.config.namespace_for_scope(scope),
            include_metadata: false,
        }
    }

    pub fn upsert_embedding_best_effort(
        &self,
        scope: &MemoryScope,
        id: &str,
        embedding: Vec<f32>,
    ) -> Result<(), String> {
        let request = self.build_upsert_request(scope, id, embedding);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let _ = self.transport.post("/vectors/upsert", &body)?;
        Ok(())
    }

    pub fn similarity_search_best_effort(
        &self,
        scope: &MemoryScope,
        query_embedding: &[f32],
    ) -> Result<Vec<VectorSearchHit>, String> {
        let request = self.build_query_request(scope, query_embedding, 100);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let raw = self.transport.post("/query", &body)?;
        Ok(parse_search_hits_from_text(&raw))
    }

    pub fn delete_embedding_best_effort(
        &self,
        scope: &MemoryScope,
        id: &str,
    ) -> Result<(), String> {
        let request = PineconeDeleteRequest {
            ids: vec![scoped_vector_id(scope, id)],
            namespace: self.config.namespace_for_scope(scope),
        };
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let _ = self.transport.post("/vectors/delete", &body)?;
        Ok(())
    }
}

impl PineconeTransport for ReqwestPineconeTransport {
    fn post(&self, path: &str, body: &str) -> Result<String, String> {
        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            let value = HeaderValue::from_str(api_key).map_err(|error| error.to_string())?;
            headers.insert("Api-Key", value);
        }
        headers.insert(
            "X-Pinecone-Api-Version",
            HeaderValue::from_static(PINECONE_API_VERSION),
        );
        self.client
            .post(format!("{}{}", self.base_url, path))
            .headers(headers)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| error.to_string())?
            .text()
            .map_err(|error| error.to_string())
    }
}

impl<T: PineconeTransport> MemoryVectorStore for PineconeVectorStoreAdapter<T> {
    fn upsert_embedding(&self, scope: &MemoryScope, id: &str, embedding: Vec<f32>) {
        let _ = self.upsert_embedding_best_effort(scope, id, embedding);
    }

    fn similarity_search(
        &self,
        scope: &MemoryScope,
        query_embedding: &[f32],
    ) -> Vec<VectorSearchHit> {
        self.similarity_search_best_effort(scope, query_embedding)
            .unwrap_or_default()
    }

    fn delete_embedding(&self, scope: &MemoryScope, id: &str) -> bool {
        self.delete_embedding_best_effort(scope, id).is_ok()
    }
}

#[allow(clippy::cast_possible_truncation)]
fn parse_search_hits_from_text(raw: &str) -> Vec<VectorSearchHit> {
    if let Ok(parsed) = serde_json::from_str::<PineconeQueryResponse>(raw) {
        return parsed
            .matches
            .into_iter()
            .map(|hit| VectorSearchHit {
                id: hit.id,
                score: hit.score,
            })
            .collect();
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(matches) = parsed.get("result").and_then(serde_json::Value::as_array) {
            return matches
                .iter()
                .filter_map(|hit| {
                    Some(VectorSearchHit {
                        id: hit.get("id")?.as_str()?.to_string(),
                        score: hit.get("score")?.as_f64()? as f32,
                    })
                })
                .collect();
        }
    }

    Vec::new()
}

fn scoped_vector_id(scope: &MemoryScope, id: &str) -> String {
    [
        scope.session_id.clone(),
        scope.repo_id.clone().unwrap_or_default(),
        scope.branch_id.clone().unwrap_or_default(),
        id.to_string(),
    ]
    .join(":")
}

fn scoped_namespace(scope: &MemoryScope) -> String {
    [
        scope.session_id.clone(),
        scope
            .repo_id
            .clone()
            .unwrap_or_else(|| "__none__".to_string()),
        scope
            .branch_id
            .clone()
            .unwrap_or_else(|| "__none__".to_string()),
    ]
    .join(":")
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{
        PineconeConfig, PineconeHttpTransportConfig, PineconeTransport, PineconeVectorStoreAdapter,
        ReqwestPineconeTransport,
    };
    use crate::{MemoryScope, MemoryVectorStore};

    #[derive(Debug, Clone, Default)]
    struct RecordingTransport {
        requests: Arc<Mutex<Vec<(String, String)>>>,
        response: Arc<Mutex<String>>,
    }

    impl RecordingTransport {
        fn with_response(response: &str) -> Self {
            Self {
                requests: Arc::new(Mutex::new(Vec::new())),
                response: Arc::new(Mutex::new(response.to_string())),
            }
        }

        fn requests(&self) -> Vec<(String, String)> {
            self.requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    impl PineconeTransport for RecordingTransport {
        fn post(&self, path: &str, body: &str) -> Result<String, String> {
            self.requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((path.to_string(), body.to_string()));
            Ok(self
                .response
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone())
        }
    }

    #[test]
    fn builds_pinecone_upsert_request_with_scoped_namespace() {
        let transport = RecordingTransport::default();
        let store = PineconeVectorStoreAdapter::new(
            PineconeConfig::new(Some("workspace".to_string())),
            transport,
        );
        let scope = MemoryScope::new("session-a")
            .with_repo_id("repo-a")
            .with_branch_id("main");

        let request = store.build_upsert_request(&scope, "memory-1", vec![0.1, 0.2]);
        assert_eq!(request.namespace, "workspace:session-a:repo-a:main");
        assert_eq!(request.vectors[0].id, "session-a:repo-a:main:memory-1");
        assert_eq!(request.vectors[0].metadata.memory_id, "memory-1");
    }

    #[test]
    fn search_parses_matches_response() {
        let transport =
            RecordingTransport::with_response(r#"{"matches":[{"id":"memory-1","score":0.93}]}"#);
        let store = PineconeVectorStoreAdapter::new(PineconeConfig::new(None), transport);
        let scope = MemoryScope::new("session-a");

        let hits = store.similarity_search(&scope, &[0.1, 0.2]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "memory-1");
        assert!((hits[0].score - 0.93).abs() < 1e-3, "score was {}", hits[0].score);
    }

    #[test]
    fn upsert_delete_and_query_use_pinecone_paths() {
        let transport =
            RecordingTransport::with_response(r#"{"matches":[{"id":"memory-1","score":0.5}]}"#);
        let store = PineconeVectorStoreAdapter::new(
            PineconeConfig::new(Some("workspace".to_string())),
            transport.clone(),
        );
        let scope = MemoryScope::new("session-a");

        store.upsert_embedding(&scope, "memory-1", vec![0.1, 0.2]);
        let _ = store.similarity_search(&scope, &[0.1, 0.2]);
        let deleted = store.delete_embedding(&scope, "memory-1");

        let requests = transport.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].0, "/vectors/upsert");
        assert_eq!(requests[1].0, "/query");
        assert_eq!(requests[2].0, "/vectors/delete");
        assert!(deleted);
    }

    #[test]
    fn reqwest_transport_trims_base_url() {
        let transport = ReqwestPineconeTransport::new(
            PineconeHttpTransportConfig::new("http://localhost:5080/").with_api_key("secret"),
        );
        let debug = format!("{transport:?}");
        assert!(debug.contains("http://localhost:5080"));
        assert!(!debug.contains("http://localhost:5080/\""));
    }
}
