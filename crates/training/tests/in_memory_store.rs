use orbit_training::{
    InMemoryStyleProfileStore, StyleDatasetBuilder, StyleProfile, StyleProfileStore, StyleSample,
    StyleScope, StyleStoreError, StyleTrainingService,
};

fn test_scope(name: &str) -> StyleScope {
    StyleScope::new(name, Some("repo-a".to_string()), None)
}

#[test]
fn in_memory_store_new() {
    let store = InMemoryStyleProfileStore::new();
    let scope = test_scope("session-new");
    let profile = store.load_profile(&scope).unwrap();
    assert!(profile.is_none());
}

#[test]
fn in_memory_store_save_and_load() {
    let store = InMemoryStyleProfileStore::new();
    let scope = test_scope("session-store");
    let profile = StyleProfile {
        sample_count: 5,
        preferred_indent: "tabs".to_string(),
        max_line_length: 120,
        snake_case_ratio: 0.8,
        notes: vec!["custom".to_string()],
    };
    store.save_profile(&scope, &profile).unwrap();
    let loaded = store.load_profile(&scope).unwrap().unwrap();
    assert_eq!(loaded, profile);
}

#[test]
fn in_memory_store_save_samples() {
    let store = InMemoryStyleProfileStore::new();
    let scope = test_scope("session-samples");
    let samples = vec![StyleSample {
        source: "test.rs".to_string(),
        content: "fn main() {}".to_string(),
    }];
    store.save_samples(&scope, &samples).unwrap();
}

#[test]
fn service_train_creates_profile() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope = test_scope("session-train");
    let samples = vec![StyleSample {
        source: "a.rs".to_string(),
        content: "fn alpha_beta() {\n    let gamma = 1;\n}\n".to_string(),
    }];
    let profile = service.train(&scope, &samples).unwrap();
    assert_eq!(profile.sample_count, 1);
    assert_eq!(profile.preferred_indent, "spaces:4");
}

#[test]
fn service_get_profile_after_train() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope = test_scope("session-get");
    let samples = vec![StyleSample {
        source: "b.rs".to_string(),
        content: "fn test() {\n    let value = 2;\n}\n".to_string(),
    }];
    let trained = service.train(&scope, &samples).unwrap();
    let loaded = service.get_profile(&scope).unwrap();
    assert_eq!(loaded, trained);
}

#[test]
fn service_get_profile_default_when_missing() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope = test_scope("session-missing");
    let profile = service.get_profile(&scope).unwrap();
    assert_eq!(profile.sample_count, 0);
}

#[allow(clippy::similar_names)]
#[test]
fn service_score() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope = test_scope("session-score");
    let samples = vec![StyleSample {
        source: "seed.rs".to_string(),
        content: "fn alpha_beta() {\n    let gamma_delta = 10;\n}\n".to_string(),
    }];
    service.train(&scope, &samples).unwrap();
    let good = "fn alpha_beta() {\n    let gamma_delta = 10;\n}\n";
    let score = service.score(&scope, good).unwrap();
    assert!(score > 0.5);
}

#[test]
fn service_scope_isolation() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope_a = StyleScope::new("session-a", Some("repo-a".to_string()), None);
    let scope_b = StyleScope::new("session-b", Some("repo-a".to_string()), None);
    let samples = vec![StyleSample {
        source: "seed.rs".to_string(),
        content: "fn alpha_beta() {\n    let gamma_delta = 10;\n}\n".to_string(),
    }];
    service.train(&scope_a, &samples).unwrap();
    let profile_a = service.get_profile(&scope_a).unwrap();
    let profile_b = service.get_profile(&scope_b).unwrap();
    assert_eq!(profile_a.sample_count, 1);
    assert_eq!(profile_b.sample_count, 0);
}

#[allow(clippy::similar_names)]
#[test]
fn scope_isolation_different_repo_ids() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope_a = StyleScope::new("session-1", Some("repo-a".to_string()), None);
    let scope_b = StyleScope::new("session-1", Some("repo-b".to_string()), None);
    let samples = vec![StyleSample {
        source: "seed.rs".to_string(),
        content: "fn x() {}".to_string(),
    }];
    service.train(&scope_a, &samples).unwrap();
    let score_a = service.score(&scope_a, "fn x() {}").unwrap();
    let _score_b = service.score(&scope_b, "fn x() {}").unwrap();
    assert!(score_a >= 0.0);
    assert_eq!(service.get_profile(&scope_b).unwrap().sample_count, 0);
}

#[test]
fn scope_isolation_different_branch_ids() {
    let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
    let scope_a = StyleScope::new(
        "session-1",
        Some("repo".to_string()),
        Some("main".to_string()),
    );
    let scope_b = StyleScope::new(
        "session-1",
        Some("repo".to_string()),
        Some("feature".to_string()),
    );
    let samples = vec![StyleSample {
        source: "seed.rs".to_string(),
        content: "fn foo() {}".to_string(),
    }];
    service.train(&scope_a, &samples).unwrap();
    assert_eq!(service.get_profile(&scope_a).unwrap().sample_count, 1);
    assert_eq!(service.get_profile(&scope_b).unwrap().sample_count, 0);
}

#[test]
fn style_dataset_builder() {
    let mut builder = StyleDatasetBuilder::new();
    builder.add_sample("a.rs", "fn a() {}");
    builder.add_sample("b.rs", "fn b() {}");
    let samples = builder.build();
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].source, "a.rs");
    assert_eq!(samples[1].source, "b.rs");
}

#[test]
fn style_dataset_builder_empty() {
    let builder = StyleDatasetBuilder::new();
    assert!(builder.build().is_empty());
}

#[test]
fn style_dataset_builder_single_sample() {
    let mut builder = StyleDatasetBuilder::new();
    builder.add_sample("test.rs", "fn test() {\n    let x = 1;\n}\n");
    let samples = builder.build();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].content, "fn test() {\n    let x = 1;\n}\n");
}

#[test]
fn style_store_error_display() {
    let err = StyleStoreError::Internal("lock poisoned".to_string());
    assert_eq!(format!("{err}"), "lock poisoned");
}

#[test]
fn style_store_error_implements_std_error() {
    let err = StyleStoreError::Internal("oops".to_string());
    let _: &dyn std::error::Error = &err;
}

#[test]
fn service_with_trainer_override() {
    let store = InMemoryStyleProfileStore::new();
    let trainer = orbit_training::StyleTrainer;
    let service = StyleTrainingService::with_trainer(store, trainer);
    let scope = test_scope("session-custom-trainer");
    let samples = vec![StyleSample {
        source: "a.rs".to_string(),
        content: "fn x() {}".to_string(),
    }];
    let profile = service.train(&scope, &samples).unwrap();
    assert_eq!(profile.sample_count, 1);
}
