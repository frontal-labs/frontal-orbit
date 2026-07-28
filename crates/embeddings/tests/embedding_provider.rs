use orbit_embeddings::{
    embed_batch_checked, embed_checked, EmbeddingError, EmbeddingModelConfig, EmbeddingModelInfo,
    EmbeddingProvider, LocalMlEmbeddingProvider,
};

#[test]
fn default_provider_has_expected_config() {
    let provider = LocalMlEmbeddingProvider::default();
    let config = provider.config();
    assert_eq!(config.model_name, "local-hash-embedding-v1");
    assert_eq!(config.dimension, 384);
    assert!(config.normalize);
}

#[test]
fn provider_with_custom_config() {
    let config = EmbeddingModelConfig {
        model_name: "custom".to_string(),
        dimension: 64,
        normalize: false,
    };
    let provider = LocalMlEmbeddingProvider::new(config);
    assert_eq!(provider.config().model_name, "custom");
    assert_eq!(provider.config().dimension, 64);
    assert!(!provider.config().normalize);
}

#[test]
fn provider_clamps_dimension_to_minimum_of_eight() {
    let config = EmbeddingModelConfig {
        model_name: "tiny".to_string(),
        dimension: 2,
        normalize: false,
    };
    let provider = LocalMlEmbeddingProvider::new(config);
    assert_eq!(provider.model_info().dimension, 8);
    let embedding = provider.embed("test");
    assert_eq!(embedding.len(), 8);
}

#[test]
fn same_text_produces_identical_embedding() {
    let provider = LocalMlEmbeddingProvider::default();
    let a = provider.embed("hello world");
    let b = provider.embed("hello world");
    assert_eq!(a, b);
}

#[test]
fn different_text_produces_different_embeddings() {
    let provider = LocalMlEmbeddingProvider::default();
    let a = provider.embed("hello world");
    let b = provider.embed("goodbye world");
    assert_ne!(a, b);
}

#[test]
fn model_info_matches_configuration() {
    let config = EmbeddingModelConfig {
        model_name: "test-model".to_string(),
        dimension: 64,
        normalize: true,
    };
    let provider = LocalMlEmbeddingProvider::new(config);
    let info = provider.model_info();
    assert_eq!(info.provider, "local-hash");
    assert_eq!(info.model_name, "test-model");
    assert_eq!(info.dimension, 64);
    assert!(info.normalize);
    assert_eq!(info.revision, None);
}

#[test]
fn model_info_from_config_constructor() {
    let config = EmbeddingModelConfig::default();
    let info = EmbeddingModelInfo::from_config("custom-provider", &config);
    assert_eq!(info.provider, "custom-provider");
    assert_eq!(info.dimension, 384);
}

#[test]
fn try_embed_returns_ok_for_valid_provider() {
    let provider = LocalMlEmbeddingProvider::default();
    let result = provider.try_embed("test input");
    assert!(result.is_ok());
    let embedding = result.unwrap();
    assert_eq!(embedding.len(), provider.model_info().dimension);
}

#[test]
fn embed_batch_produces_correct_number_of_embeddings() {
    let provider = LocalMlEmbeddingProvider::default();
    let texts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let embeddings = provider.embed_batch(&texts);
    assert_eq!(embeddings.len(), 3);
    for embedding in &embeddings {
        assert_eq!(embedding.len(), provider.model_info().dimension);
    }
}

#[test]
fn embed_batch_with_empty_input() {
    let provider = LocalMlEmbeddingProvider::default();
    let embeddings = provider.embed_batch(&[]);
    assert!(embeddings.is_empty());
}

#[test]
fn try_embed_batch_succeeds() {
    let provider = LocalMlEmbeddingProvider::default();
    let texts = vec!["x".to_string(), "y".to_string()];
    let result = provider.try_embed_batch(&texts);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn embed_checked_validates_successfully() {
    let provider = LocalMlEmbeddingProvider::default();
    let result = embed_checked(&provider, "test");
    assert!(result.is_ok());
}

#[test]
fn embed_batch_checked_validates_all_embeddings() {
    let provider = LocalMlEmbeddingProvider::default();
    let texts = vec!["a".to_string(), "b".to_string()];
    let result = embed_batch_checked(&provider, &texts);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn embed_error_display_for_dimension_mismatch() {
    let mismatch = EmbeddingError::DimensionMismatch {
        expected: 384,
        actual: 16,
    };
    let msg = mismatch.to_string();
    assert!(msg.contains("384"));
    assert!(msg.contains("16"));
}

#[test]
fn embed_error_display_for_config_and_provider() {
    let invalid = EmbeddingError::InvalidConfig("bad config".to_string());
    let provider_err = EmbeddingError::Provider("backend down".to_string());
    assert_eq!(invalid.to_string(), "bad config");
    assert_eq!(provider_err.to_string(), "backend down");
}

#[test]
fn embed_checked_rejects_empty_model_name() {
    let provider = LocalMlEmbeddingProvider::new(EmbeddingModelConfig {
        model_name: String::new(),
        dimension: 8,
        normalize: false,
    });
    let result = embed_checked(&provider, "test");
    assert!(result.is_err());
    match result.unwrap_err() {
        EmbeddingError::InvalidConfig(msg) => {
            assert!(msg.contains("model name must not be empty"));
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn embedding_is_l2_normalized_when_normalize_is_true() {
    let provider = LocalMlEmbeddingProvider::default();
    assert!(provider.config().normalize);
    let embedding = provider.embed("normalize me");
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-5, "expected norm ~1.0, got {norm}");
}

#[test]
fn embedding_is_not_normalized_when_normalize_is_false() {
    let config = EmbeddingModelConfig {
        model_name: "raw".to_string(),
        dimension: 8,
        normalize: false,
    };
    let provider = LocalMlEmbeddingProvider::new(config);
    let embedding = provider.embed("raw embedding");
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(norm > 0.0, "expected non-zero norm");
}

#[test]
fn embed_and_try_embed_are_consistent() {
    let provider = LocalMlEmbeddingProvider::default();
    let text = "some code: fn main() -> Result<(), Error> { Ok(()) }";
    let legacy = provider.embed(text);
    let fallible = provider.try_embed(text).expect("should succeed");
    assert_eq!(legacy, fallible);
}

#[test]
fn empty_text_still_produces_embedding() {
    let provider = LocalMlEmbeddingProvider::default();
    let embedding = provider.embed("");
    assert_eq!(embedding.len(), provider.model_info().dimension);
}

#[test]
fn whitespace_text_still_produces_embedding() {
    let provider = LocalMlEmbeddingProvider::default();
    let embedding = provider.embed("   \n  \t  ");
    assert_eq!(embedding.len(), provider.model_info().dimension);
}

#[test]
fn tokenization_splits_on_non_alphanumeric_characters() {
    let provider = LocalMlEmbeddingProvider::default();
    let a = provider.embed("hello-world");
    let b = provider.embed("hello.world");
    let c = provider.embed("hello world");
    assert_eq!(a, b, "tokenization should treat - and . as delimiters");
    assert_eq!(b, c, "tokenization should treat space as delimiter");
}

#[test]
fn tokenization_is_case_insensitive() {
    let provider = LocalMlEmbeddingProvider::default();
    let lower = provider.embed("hello world");
    let upper = provider.embed("HELLO WORLD");
    let mixed = provider.embed("Hello World");
    assert_eq!(lower, upper, "tokenization should lowercase");
    assert_eq!(lower, mixed, "tokenization should lowercase");
}
