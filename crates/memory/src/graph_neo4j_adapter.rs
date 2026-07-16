//! Neo4j-shaped HTTP adapter for `MemoryGraphStore`.
//!
//! This module is intentionally transport-agnostic and focuses on query/data
//! shape definitions plus best-effort adapter methods.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{KgEntity, KgRelation, MemoryGraphStore, MemoryScope};

/// Minimal transport boundary for Neo4j HTTP calls.
pub trait Neo4jTransport: Send + Sync {
    fn post(&self, path: &str, body: &str) -> Result<String, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neo4jConfig {
    pub database: String,
}

impl Neo4jConfig {
    #[must_use]
    pub fn new(database: impl Into<String>) -> Self {
        Self {
            database: database.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neo4jHttpTransportConfig {
    pub base_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Neo4jHttpTransportConfig {
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            username: None,
            password: None,
        }
    }

    #[must_use]
    pub fn with_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct ReqwestNeo4jTransport {
    base_url: String,
    username: Option<String>,
    password: Option<String>,
    client: Client,
}

impl ReqwestNeo4jTransport {
    #[must_use]
    pub fn new(config: Neo4jHttpTransportConfig) -> Self {
        Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            username: config.username,
            password: config.password,
            client: Client::new(),
        }
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neo4jStatement {
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neo4jTransactionRequest {
    pub statements: Vec<Neo4jStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neo4jTransactionResponse {
    pub results: Vec<Neo4jResult>,
    pub errors: Vec<Neo4jError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neo4jResult {
    pub columns: Vec<String>,
    pub data: Vec<Neo4jResultRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neo4jResultRow {
    pub row: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Neo4jError {
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub struct Neo4jGraphStoreAdapter<T: Neo4jTransport> {
    config: Neo4jConfig,
    transport: T,
}

impl<T: Neo4jTransport> Neo4jGraphStoreAdapter<T> {
    #[must_use]
    pub fn new(config: Neo4jConfig, transport: T) -> Self {
        Self { config, transport }
    }

    #[must_use]
    pub fn build_upsert_entity_statement(scope: &MemoryScope, entity: &KgEntity) -> String {
        format!(
            "MERGE (n:Entity {{scope_key: '{scope_key}', id: '{id}'}}) \
             SET n.label = '{label}', n.entity_type = '{entity_type}'",
            scope_key = scoped_key(scope),
            id = escape_cypher_string(&entity.id),
            label = escape_cypher_string(&entity.label),
            entity_type = escape_cypher_string(&entity.entity_type),
        )
    }

    #[must_use]
    pub fn build_add_relation_statement(scope: &MemoryScope, relation: &KgRelation) -> String {
        format!(
            "MATCH (s:Entity {{scope_key: '{scope_key}', id: '{subject}'}}), \
             (o:Entity {{scope_key: '{scope_key}', id: '{object}'}}) \
             MERGE (s)-[r:RELATED {{predicate: '{predicate}'}}]->(o)",
            scope_key = scoped_key(scope),
            subject = escape_cypher_string(&relation.subject_id),
            object = escape_cypher_string(&relation.object_id),
            predicate = escape_cypher_string(&relation.predicate),
        )
    }

    #[must_use]
    pub fn build_neighbors_statement(scope: &MemoryScope, entity_id: &str) -> String {
        format!(
            "MATCH (s:Entity {{scope_key: '{scope_key}'}})-[r:RELATED]-(o:Entity {{scope_key: '{scope_key}'}}) \
             WHERE s.id = '{entity_id}' OR o.id = '{entity_id}' \
             RETURN s.id, r.predicate, o.id",
            scope_key = scoped_key(scope),
            entity_id = escape_cypher_string(entity_id),
        )
    }

    #[must_use]
    pub fn build_list_entities_statement(scope: &MemoryScope) -> String {
        format!(
            "MATCH (n:Entity {{scope_key: '{scope_key}'}}) RETURN n.id, n.label, n.entity_type",
            scope_key = scoped_key(scope),
        )
    }

    #[must_use]
    pub fn build_list_relations_statement(scope: &MemoryScope) -> String {
        format!(
            "MATCH (s:Entity {{scope_key: '{scope_key}'}})-[r:RELATED]->(o:Entity {{scope_key: '{scope_key}'}}) RETURN s.id, r.predicate, o.id",
            scope_key = scoped_key(scope),
        )
    }

    #[must_use]
    pub fn build_delete_entity_statement(scope: &MemoryScope, entity_id: &str) -> String {
        format!(
            "MATCH (n:Entity {{scope_key: '{scope_key}', id: '{entity_id}'}}) DETACH DELETE n",
            scope_key = scoped_key(scope),
            entity_id = escape_cypher_string(entity_id),
        )
    }

    #[must_use]
    pub fn build_delete_relation_statement(scope: &MemoryScope, relation: &KgRelation) -> String {
        format!(
            "MATCH (s:Entity {{scope_key: '{scope_key}', id: '{subject}'}})-[r:RELATED {{predicate: '{predicate}'}}]->(o:Entity {{scope_key: '{scope_key}', id: '{object}'}}) DELETE r",
            scope_key = scoped_key(scope),
            subject = escape_cypher_string(&relation.subject_id),
            predicate = escape_cypher_string(&relation.predicate),
            object = escape_cypher_string(&relation.object_id),
        )
    }

    pub fn upsert_entity_best_effort(
        &self,
        scope: &MemoryScope,
        entity: &KgEntity,
    ) -> Result<(), String> {
        let statement = Self::build_upsert_entity_statement(scope, entity);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let _ = self.transport.post(&path, &body)?;
        Ok(())
    }

    pub fn add_relation_best_effort(
        &self,
        scope: &MemoryScope,
        relation: &KgRelation,
    ) -> Result<(), String> {
        let statement = Self::build_add_relation_statement(scope, relation);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let _ = self.transport.post(&path, &body)?;
        Ok(())
    }

    pub fn neighbors_best_effort(
        &self,
        scope: &MemoryScope,
        entity_id: &str,
    ) -> Result<Vec<KgRelation>, String> {
        let statement = Self::build_neighbors_statement(scope, entity_id);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let raw = self.transport.post(&path, &body)?;
        Ok(parse_relations_from_text(&raw))
    }

    pub fn list_entities_best_effort(&self, scope: &MemoryScope) -> Result<Vec<KgEntity>, String> {
        let statement = Self::build_list_entities_statement(scope);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let raw = self.transport.post(&path, &body)?;
        Ok(parse_entities_from_text(&raw))
    }

    pub fn list_relations_best_effort(
        &self,
        scope: &MemoryScope,
    ) -> Result<Vec<KgRelation>, String> {
        let statement = Self::build_list_relations_statement(scope);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let raw = self.transport.post(&path, &body)?;
        Ok(parse_relations_from_text(&raw))
    }

    pub fn delete_entity_best_effort(
        &self,
        scope: &MemoryScope,
        entity_id: &str,
    ) -> Result<(), String> {
        let statement = Self::build_delete_entity_statement(scope, entity_id);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let _ = self.transport.post(&path, &body)?;
        Ok(())
    }

    pub fn delete_relation_best_effort(
        &self,
        scope: &MemoryScope,
        relation: &KgRelation,
    ) -> Result<(), String> {
        let statement = Self::build_delete_relation_statement(scope, relation);
        let request = Neo4jTransactionRequest {
            statements: vec![Neo4jStatement { statement }],
        };
        let path = format!("/db/{}/tx/commit", self.config.database);
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let _ = self.transport.post(&path, &body)?;
        Ok(())
    }
}

impl Neo4jTransport for ReqwestNeo4jTransport {
    fn post(&self, path: &str, body: &str) -> Result<String, String> {
        let mut request = self
            .client
            .post(self.build_url(path))
            .header("content-type", "application/json")
            .body(body.to_string());
        if let Some(username) = &self.username {
            request = request.basic_auth(username, self.password.clone());
        }
        request
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| error.to_string())?
            .text()
            .map_err(|error| error.to_string())
    }
}

impl<T: Neo4jTransport> MemoryGraphStore for Neo4jGraphStoreAdapter<T> {
    fn upsert_entity(&self, scope: &MemoryScope, entity: KgEntity) {
        let _ = self.upsert_entity_best_effort(scope, &entity);
    }

    fn add_relation(&self, scope: &MemoryScope, relation: KgRelation) {
        let _ = self.add_relation_best_effort(scope, &relation);
    }

    fn list_entities(&self, scope: &MemoryScope) -> Vec<KgEntity> {
        self.list_entities_best_effort(scope).unwrap_or_default()
    }

    fn list_relations(&self, scope: &MemoryScope) -> Vec<KgRelation> {
        self.list_relations_best_effort(scope).unwrap_or_default()
    }

    fn neighbors(&self, scope: &MemoryScope, entity_id: &str) -> Vec<KgRelation> {
        self.neighbors_best_effort(scope, entity_id)
            .unwrap_or_default()
    }

    fn delete_entity(&self, scope: &MemoryScope, entity_id: &str) -> bool {
        self.delete_entity_best_effort(scope, entity_id).is_ok()
    }

    fn delete_relation(&self, scope: &MemoryScope, relation: &KgRelation) -> bool {
        self.delete_relation_best_effort(scope, relation).is_ok()
    }
}

fn scoped_key(scope: &MemoryScope) -> String {
    format!(
        "{}:{}:{}",
        scope.session_id,
        scope.repo_id.as_deref().unwrap_or_default(),
        scope.branch_id.as_deref().unwrap_or_default()
    )
}

fn escape_cypher_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn parse_entities_from_text(raw: &str) -> Vec<KgEntity> {
    if let Ok(parsed) = serde_json::from_str::<Neo4jTransactionResponse>(raw) {
        let mut entities = Vec::new();
        for result in parsed.results {
            for row in result.data {
                if row.row.len() != 3 {
                    continue;
                }
                entities.push(KgEntity {
                    id: row.row[0].clone(),
                    label: row.row[1].clone(),
                    entity_type: row.row[2].clone(),
                });
            }
        }
        if !entities.is_empty() {
            return entities;
        }
    }

    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split('|');
            let id = parts.next()?.trim().to_string();
            let label = parts.next()?.trim().to_string();
            let entity_type = parts.next()?.trim().to_string();
            if id.is_empty() || label.is_empty() || entity_type.is_empty() {
                return None;
            }
            Some(KgEntity {
                id,
                label,
                entity_type,
            })
        })
        .collect()
}

fn parse_relations_from_text(raw: &str) -> Vec<KgRelation> {
    if let Ok(parsed) = serde_json::from_str::<Neo4jTransactionResponse>(raw) {
        let mut relations = Vec::new();
        for result in parsed.results {
            for row in result.data {
                if row.row.len() != 3 {
                    continue;
                }
                relations.push(KgRelation {
                    subject_id: row.row[0].clone(),
                    predicate: row.row[1].clone(),
                    object_id: row.row[2].clone(),
                });
            }
        }
        if !relations.is_empty() {
            return relations;
        }
    }

    raw.lines()
        .filter_map(|line| {
            // Lightweight parse shape: `subject|predicate|object`
            let mut parts = line.split('|');
            let subject_id = parts.next()?.trim().to_string();
            let predicate = parts.next()?.trim().to_string();
            let object_id = parts.next()?.trim().to_string();
            if subject_id.is_empty() || predicate.is_empty() || object_id.is_empty() {
                return None;
            }
            Some(KgRelation {
                subject_id,
                predicate,
                object_id,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::thread;
    use std::time::Duration;

    use super::{
        Neo4jConfig, Neo4jGraphStoreAdapter, Neo4jHttpTransportConfig, Neo4jTransport,
        ReqwestNeo4jTransport,
    };
    use crate::{KgEntity, KgRelation, MemoryGraphStore, MemoryScope};

    #[derive(Debug, Clone, Default)]
    struct RecordingTransport {
        calls: Arc<Mutex<Vec<(String, String)>>>,
        response: Arc<Mutex<String>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestHttpRequest {
        request_line: String,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl TestHttpRequest {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }
    }

    struct TestServer {
        addr: SocketAddr,
        shutdown: Option<std::sync::mpsc::Sender<()>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn spawn(
            handler: Arc<dyn Fn(&TestHttpRequest) -> HttpResponse + Send + Sync + 'static>,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            listener
                .set_nonblocking(true)
                .expect("set nonblocking listener");
            let addr = listener.local_addr().expect("local addr");
            let (tx, rx) = std::sync::mpsc::channel::<()>();

            let handle = thread::spawn(move || loop {
                if rx.try_recv().is_ok() {
                    break;
                }

                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_http_request(&mut stream);
                        let response = handler(&request);
                        stream
                            .write_all(response.to_bytes().as_slice())
                            .expect("write response");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("server accept failed: {error}"),
                }
            });

            Self {
                addr,
                shutdown: Some(tx),
                handle: Some(handle),
            }
        }

        fn base_url(&self) -> String {
            format!("http://{}", self.addr)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            if let Some(tx) = self.shutdown.take() {
                let _ = tx.send(());
            }
            if let Some(handle) = self.handle.take() {
                handle.join().expect("join test server");
            }
        }
    }

    fn loopback_bind_available() -> bool {
        static AVAILABLE: OnceLock<bool> = OnceLock::new();
        *AVAILABLE.get_or_init(|| match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => {
                drop(listener);
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("failed to probe loopback bind availability: {error}"),
        })
    }

    struct HttpResponse {
        status: u16,
        reason: &'static str,
        content_type: &'static str,
        body: String,
    }

    impl HttpResponse {
        fn json(body: &str) -> Self {
            Self {
                status: 200,
                reason: "OK",
                content_type: "application/json",
                body: body.to_string(),
            }
        }

        fn to_bytes(&self) -> Vec<u8> {
            format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                self.status,
                self.reason,
                self.content_type,
                self.body.len(),
                self.body
            )
            .into_bytes()
        }
    }

    fn read_http_request(stream: &mut TcpStream) -> TestHttpRequest {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];

        loop {
            let size = stream.read(&mut buffer).expect("read request");
            if size == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..size]);
            if let Some(headers_end) = find_headers_end(&bytes) {
                let header_text = String::from_utf8_lossy(&bytes[..headers_end]).into_owned();
                let content_length = parse_content_length(&header_text);
                let body_start = headers_end + 4;
                if bytes.len() >= body_start + content_length {
                    return parse_request(&bytes[..body_start + content_length]);
                }
            }
        }

        parse_request(&bytes)
    }

    fn find_headers_end(bytes: &[u8]) -> Option<usize> {
        bytes.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn parse_content_length(header_text: &str) -> usize {
        header_text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn parse_request(bytes: &[u8]) -> TestHttpRequest {
        let request = String::from_utf8_lossy(bytes).into_owned();
        let (header_text, body) = request.split_once("\r\n\r\n").unwrap_or((&request, ""));
        let mut lines = header_text.lines();
        let request_line = lines.next().unwrap_or_default().to_string();
        let headers = lines
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                Some((name.trim().to_string(), value.trim().to_string()))
            })
            .collect();
        TestHttpRequest {
            request_line,
            headers,
            body: body.to_string(),
        }
    }

    impl RecordingTransport {
        fn with_response(response: &str) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                response: Arc::new(Mutex::new(response.to_string())),
            }
        }

        fn calls(&self) -> Vec<(String, String)> {
            self.calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    impl Neo4jTransport for RecordingTransport {
        fn post(&self, path: &str, body: &str) -> Result<String, String> {
            self.calls
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
    fn builds_scope_aware_entity_upsert_query() {
        let transport = RecordingTransport::default();
        let store = Neo4jGraphStoreAdapter::new(Neo4jConfig::new("neo4j"), transport.clone());
        let scope = MemoryScope::new("session-a")
            .with_repo_id("repo-a")
            .with_branch_id("main");
        let entity = KgEntity {
            id: "crate:memory".to_string(),
            label: "orbit-memory".to_string(),
            entity_type: "crate".to_string(),
        };

        store.upsert_entity(&scope, entity);

        let calls = transport.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "/db/neo4j/tx/commit");
        assert!(calls[0].1.contains("session-a:repo-a:main"));
        assert!(calls[0].1.contains("crate:memory"));
    }

    #[test]
    fn parses_best_effort_neighbor_rows() {
        let transport = RecordingTransport::with_response(
            "crate:tools|depends_on|crate:memory\ncrate:runtime|uses|crate:memory\n",
        );
        let store = Neo4jGraphStoreAdapter::new(Neo4jConfig::new("neo4j"), transport);
        let scope = MemoryScope::new("session-a");

        let neighbors = store.neighbors(&scope, "crate:memory");
        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors[0].predicate, "depends_on");
        assert_eq!(neighbors[1].predicate, "uses");
    }

    #[test]
    fn add_relation_emits_expected_cypher_shape() {
        let transport = RecordingTransport::default();
        let store = Neo4jGraphStoreAdapter::new(Neo4jConfig::new("neo4j"), transport.clone());
        let scope = MemoryScope::new("session-a");
        store.add_relation(
            &scope,
            KgRelation {
                subject_id: "crate:tools".to_string(),
                predicate: "depends_on".to_string(),
                object_id: "crate:memory".to_string(),
            },
        );

        let calls = transport.calls();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].1.contains("MERGE (s)-[r:RELATED"));
        assert!(calls[0].1.contains("depends_on"));
    }

    #[test]
    fn parses_json_neighbor_rows() {
        let json_response = r#"{
            "results": [{
                "columns": ["subject", "predicate", "object"],
                "data": [{
                    "row": ["crate:tools", "depends_on", "crate:memory"]
                }]
            }],
            "errors": []
        }"#;
        let transport = RecordingTransport::with_response(json_response);
        let store = Neo4jGraphStoreAdapter::new(Neo4jConfig::new("neo4j"), transport);
        let scope = MemoryScope::new("session-a");

        let neighbors = store.neighbors(&scope, "crate:memory");
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].predicate, "depends_on");
    }

    #[test]
    fn reqwest_transport_builds_url_without_trailing_slash() {
        let config = Neo4jHttpTransportConfig::new("http://localhost:7474/");
        let transport = ReqwestNeo4jTransport::new(config);
        assert_eq!(
            transport.build_url("/db://example"),
            "http://localhost:7474/db://example"
        );
    }

    #[test]
    fn reqwest_transport_posts_expected_neo4j_requests() {
        if !loopback_bind_available() {
            return;
        }
        let calls = Arc::new(Mutex::new(Vec::<TestHttpRequest>::new()));
        let recorded_calls = calls.clone();
        let server = TestServer::spawn(Arc::new(move |request| {
            recorded_calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(request.clone());
            HttpResponse::json(
                r#"{"results":[{"columns":["s.id","r.predicate","o.id"],"data":[{"row":["crate:tools","depends_on","crate:memory"]}]}],"errors":[]}"#,
            )
        }));

        let transport = ReqwestNeo4jTransport::new(
            Neo4jHttpTransportConfig::new(format!("{}/", server.base_url()))
                .with_basic_auth("neo4j", "secret"),
        );
        let store = Neo4jGraphStoreAdapter::new(Neo4jConfig::new("neo4j"), transport);
        let scope = MemoryScope::new("session-a");

        store.upsert_entity(
            &scope,
            KgEntity {
                id: "crate:tools".to_string(),
                label: "orbit-tools".to_string(),
                entity_type: "crate".to_string(),
            },
        );
        store.add_relation(
            &scope,
            KgRelation {
                subject_id: "crate:tools".to_string(),
                predicate: "depends_on".to_string(),
                object_id: "crate:memory".to_string(),
            },
        );
        let neighbors = store.neighbors(&scope, "crate:memory");

        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].predicate, "depends_on");

        let calls = calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].request_line, "POST /db/neo4j/tx/commit HTTP/1.1");
        assert!(calls[0].body.contains("MERGE (n:Entity"));
        assert!(calls[1].body.contains("MERGE (s)-[r:RELATED"));
        assert!(calls[2].body.contains("RETURN s.id, r.predicate, o.id"));
        assert!(calls[0].header("authorization").is_some());
    }
}
