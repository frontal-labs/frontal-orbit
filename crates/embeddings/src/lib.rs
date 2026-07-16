//! # Orbit Embeddings
//!
//! Embedding primitives and a lightweight local embedding provider used by
//! semantic memory and style-learning workflows.

use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingModelConfig {
    pub model_name: String,
    pub dimension: usize,
    pub normalize: bool,
}

impl Default for EmbeddingModelConfig {
    fn default() -> Self {
        Self {
            model_name: "local-hash-embedding-v1".to_string(),
            dimension: 384,
            normalize: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingModelInfo {
    pub provider: String,
    pub model_name: String,
    pub dimension: usize,
    pub normalize: bool,
    pub revision: Option<String>,
}

impl EmbeddingModelInfo {
    #[must_use]
    pub fn from_config(provider: impl Into<String>, config: &EmbeddingModelConfig) -> Self {
        Self {
            provider: provider.into(),
            model_name: config.model_name.clone(),
            dimension: config.dimension.max(8),
            normalize: config.normalize,
            revision: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EmbeddingError {
    InvalidConfig(String),
    Provider(String),
    DimensionMismatch { expected: usize, actual: usize },
}

impl Display for EmbeddingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(message) | Self::Provider(message) => write!(f, "{message}"),
            Self::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "embedding dimension mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for EmbeddingError {}

pub trait EmbeddingProvider: Send + Sync {
    fn config(&self) -> &EmbeddingModelConfig;

    #[must_use]
    fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo::from_config("unknown", self.config())
    }

    fn try_embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.embed(text))
    }

    fn embed(&self, text: &str) -> Vec<f32>;

    fn try_embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        texts
            .iter()
            .map(|text| self.try_embed(text))
            .collect::<Result<Vec<_>, _>>()
    }

    fn embed_batch(&self, texts: &[String]) -> Vec<Vec<f32>> {
        texts.iter().map(|text| self.embed(text)).collect()
    }
}

#[derive(Debug, Clone)]
pub struct LocalMlEmbeddingProvider {
    config: EmbeddingModelConfig,
    model_info: EmbeddingModelInfo,
}

impl LocalMlEmbeddingProvider {
    #[must_use]
    pub fn new(config: EmbeddingModelConfig) -> Self {
        let model_info = EmbeddingModelInfo::from_config("local-hash", &config);
        Self { config, model_info }
    }
}

impl Default for LocalMlEmbeddingProvider {
    fn default() -> Self {
        Self::new(EmbeddingModelConfig::default())
    }
}

impl EmbeddingProvider for LocalMlEmbeddingProvider {
    fn config(&self) -> &EmbeddingModelConfig {
        &self.config
    }

    fn model_info(&self) -> EmbeddingModelInfo {
        self.model_info.clone()
    }

    fn try_embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        validate_model_info(&self.model_info)?;
        let embedding = self.embed(text);
        validate_embedding_dimension(&embedding, self.model_info.dimension)?;
        Ok(embedding)
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let dim = self.config.dimension.max(8);
        let mut vector = vec![0.0_f32; dim];

        for token in tokenize(text) {
            let mut hasher = DefaultHasher::new();
            token.hash(&mut hasher);
            let hash = hasher.finish();
            let dim_u64 = dim as u64;
            let index = usize::try_from(hash % dim_u64).unwrap_or(0);
            let sign = if hash & 1 == 0 { 1.0_f32 } else { -1.0_f32 };
            vector[index] += sign;
        }

        if self.config.normalize {
            normalize_l2(&mut vector);
        }

        vector
    }
}

pub fn embed_checked(
    provider: &dyn EmbeddingProvider,
    text: &str,
) -> Result<Vec<f32>, EmbeddingError> {
    let info = provider.model_info();
    validate_model_info(&info)?;
    let embedding = provider.try_embed(text)?;
    validate_embedding_dimension(&embedding, info.dimension)?;
    Ok(embedding)
}

pub fn embed_batch_checked(
    provider: &dyn EmbeddingProvider,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, EmbeddingError> {
    let info = provider.model_info();
    validate_model_info(&info)?;
    let embeddings = provider.try_embed_batch(texts)?;
    for embedding in &embeddings {
        validate_embedding_dimension(embedding, info.dimension)?;
    }
    Ok(embeddings)
}

#[must_use]
pub fn cosine_similarity(lhs: &[f32], rhs: &[f32]) -> f32 {
    if lhs.len() != rhs.len() || lhs.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut lhs_norm = 0.0_f32;
    let mut rhs_norm = 0.0_f32;

    for (&l, &r) in lhs.iter().zip(rhs.iter()) {
        dot += l * r;
        lhs_norm += l * l;
        rhs_norm += r * r;
    }

    if lhs_norm <= f32::EPSILON || rhs_norm <= f32::EPSILON {
        return 0.0;
    }

    dot / (lhs_norm.sqrt() * rhs_norm.sqrt())
}

#[must_use]
pub fn top_k_by_similarity(
    query: &[f32],
    candidates: impl IntoIterator<Item = (String, Vec<f32>)>,
    k: usize,
) -> Vec<(String, f32)> {
    let mut scored = candidates
        .into_iter()
        .map(|(id, embedding)| {
            let score = cosine_similarity(query, &embedding);
            (id, score)
        })
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(k);
    scored
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
}

fn normalize_l2(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return;
    }

    for value in vector.iter_mut() {
        *value /= norm;
    }
}

fn validate_model_info(info: &EmbeddingModelInfo) -> Result<(), EmbeddingError> {
    if info.dimension == 0 {
        return Err(EmbeddingError::InvalidConfig(
            "embedding dimension must be greater than zero".to_string(),
        ));
    }
    if info.model_name.trim().is_empty() {
        return Err(EmbeddingError::InvalidConfig(
            "embedding model name must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_embedding_dimension(
    embedding: &[f32],
    expected_dimension: usize,
) -> Result<(), EmbeddingError> {
    if embedding.len() != expected_dimension {
        return Err(EmbeddingError::DimensionMismatch {
            expected: expected_dimension,
            actual: embedding.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        cosine_similarity, embed_batch_checked, embed_checked, EmbeddingError,
        EmbeddingModelConfig, EmbeddingModelInfo, EmbeddingProvider, LocalMlEmbeddingProvider,
    };

    #[derive(Debug, Clone)]
    struct FailingProvider {
        config: EmbeddingModelConfig,
    }

    impl FailingProvider {
        fn new(config: EmbeddingModelConfig) -> Self {
            Self { config }
        }
    }

    impl EmbeddingProvider for FailingProvider {
        fn config(&self) -> &EmbeddingModelConfig {
            &self.config
        }

        fn try_embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
            Err(EmbeddingError::Provider(
                "simulated provider backend failure".to_string(),
            ))
        }

        fn embed(&self, _text: &str) -> Vec<f32> {
            vec![0.0_f32; self.config.dimension.max(8)]
        }
    }

    #[derive(Debug, Clone)]
    struct MismatchedProvider {
        config: EmbeddingModelConfig,
    }

    impl MismatchedProvider {
        fn new(config: EmbeddingModelConfig) -> Self {
            Self { config }
        }
    }

    impl EmbeddingProvider for MismatchedProvider {
        fn config(&self) -> &EmbeddingModelConfig {
            &self.config
        }

        fn model_info(&self) -> EmbeddingModelInfo {
            EmbeddingModelInfo {
                provider: "test".to_string(),
                model_name: "mismatch".to_string(),
                dimension: 32,
                normalize: false,
                revision: None,
            }
        }

        fn try_embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
            Ok(vec![0.0_f32; 16])
        }

        fn embed(&self, _text: &str) -> Vec<f32> {
            vec![0.0_f32; 16]
        }
    }

    #[test]
    fn same_text_is_most_similar() {
        let provider = LocalMlEmbeddingProvider::default();
        let a = provider.embed("fn parse_args(input: &str) -> Result<(), Error>");
        let b = provider.embed("fn parse_args(input: &str) -> Result<(), Error>");
        let c = provider.embed("docker compose up postgres redis pinecone");

        let same = cosine_similarity(&a, &b);
        let different = cosine_similarity(&a, &c);
        assert!(same > different);
    }

    #[test]
    fn embedding_dimension_matches_config() {
        let provider = LocalMlEmbeddingProvider::default();
        let embedding = provider.embed("let greeting = \"hello\";");
        assert_eq!(embedding.len(), provider.model_info().dimension);
    }

    #[test]
    fn model_info_matches_local_provider_configuration() {
        let provider = LocalMlEmbeddingProvider::default();
        let info = provider.model_info();
        assert_eq!(info.provider, "local-hash");
        assert_eq!(info.model_name, provider.config().model_name);
        assert_eq!(info.dimension, provider.config().dimension.max(8));
        assert!(info.normalize);
    }

    #[test]
    fn try_embed_is_compatible_with_embed() {
        let provider = LocalMlEmbeddingProvider::default();
        let text = "fn parse_args(input: &str) -> Result<(), Error>";
        let legacy = provider.embed(text);
        let fallible = provider
            .try_embed(text)
            .expect("local embed should succeed");
        assert_eq!(legacy, fallible);
    }

    #[test]
    fn checked_embed_surfaces_provider_errors() {
        let provider = FailingProvider::new(EmbeddingModelConfig::default());
        let error = embed_checked(&provider, "hello").expect_err("provider should fail");
        assert!(matches!(error, EmbeddingError::Provider(_)));
    }

    #[test]
    fn checked_embed_surfaces_dimension_mismatches() {
        let provider = MismatchedProvider::new(EmbeddingModelConfig::default());
        let error = embed_checked(&provider, "hello").expect_err("dimension check should fail");
        assert!(matches!(
            error,
            EmbeddingError::DimensionMismatch {
                expected: 32,
                actual: 16
            }
        ));
    }

    #[test]
    fn checked_embed_batch_validates_all_embeddings() {
        let provider = LocalMlEmbeddingProvider::default();
        let embeddings = embed_batch_checked(
            &provider,
            &["a".to_string(), "b".to_string(), "c".to_string()],
        )
        .expect("batch embed should succeed");
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), provider.model_info().dimension);
    }
}
