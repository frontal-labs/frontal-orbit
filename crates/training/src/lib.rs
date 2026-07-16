//! # Orbit Training
//!
//! Style-learning pipeline for coding conventions.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StyleSample {
    pub source: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleProfile {
    pub sample_count: usize,
    pub preferred_indent: String,
    pub max_line_length: usize,
    pub snake_case_ratio: f32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct StyleScope {
    pub session_id: String,
    pub repo_id: Option<String>,
    pub branch_id: Option<String>,
}

impl StyleScope {
    #[must_use]
    pub fn new(
        session_id: impl Into<String>,
        repo_id: Option<String>,
        branch_id: Option<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            repo_id,
            branch_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleStoreError {
    Internal(String),
}

impl Display for StyleStoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for StyleStoreError {}

pub trait StyleProfileStore: Send + Sync {
    fn load_profile(&self, scope: &StyleScope) -> Result<Option<StyleProfile>, StyleStoreError>;

    fn save_profile(
        &self,
        scope: &StyleScope,
        profile: &StyleProfile,
    ) -> Result<(), StyleStoreError>;

    fn save_samples(
        &self,
        scope: &StyleScope,
        samples: &[StyleSample],
    ) -> Result<(), StyleStoreError>;
}

#[derive(Debug, Default)]
struct InMemoryStyleStoreState {
    profiles: BTreeMap<StyleScope, StyleProfile>,
    samples: BTreeMap<StyleScope, Vec<StyleSample>>,
}

#[derive(Debug, Default)]
pub struct InMemoryStyleProfileStore {
    inner: RwLock<InMemoryStyleStoreState>,
}

impl InMemoryStyleProfileStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StyleProfileStore for InMemoryStyleProfileStore {
    fn load_profile(&self, scope: &StyleScope) -> Result<Option<StyleProfile>, StyleStoreError> {
        let state = self
            .inner
            .read()
            .map_err(|_| StyleStoreError::Internal("style store lock poisoned".to_string()))?;
        Ok(state.profiles.get(scope).cloned())
    }

    fn save_profile(
        &self,
        scope: &StyleScope,
        profile: &StyleProfile,
    ) -> Result<(), StyleStoreError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| StyleStoreError::Internal("style store lock poisoned".to_string()))?;
        state.profiles.insert(scope.clone(), profile.clone());
        Ok(())
    }

    fn save_samples(
        &self,
        scope: &StyleScope,
        samples: &[StyleSample],
    ) -> Result<(), StyleStoreError> {
        let mut state = self
            .inner
            .write()
            .map_err(|_| StyleStoreError::Internal("style store lock poisoned".to_string()))?;
        state.samples.insert(scope.clone(), samples.to_vec());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StyleTrainingService<S> {
    trainer: StyleTrainer,
    store: S,
}

impl<S> StyleTrainingService<S>
where
    S: StyleProfileStore,
{
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            trainer: StyleTrainer,
            store,
        }
    }

    #[must_use]
    pub fn with_trainer(store: S, trainer: StyleTrainer) -> Self {
        Self { trainer, store }
    }

    pub fn train(
        &self,
        scope: &StyleScope,
        samples: &[StyleSample],
    ) -> Result<StyleProfile, StyleStoreError> {
        self.store.save_samples(scope, samples)?;
        let profile = self.trainer.train(samples);
        self.store.save_profile(scope, &profile)?;
        Ok(profile)
    }

    pub fn get_profile(&self, scope: &StyleScope) -> Result<StyleProfile, StyleStoreError> {
        Ok(self.store.load_profile(scope)?.unwrap_or_default())
    }

    pub fn score(&self, scope: &StyleScope, candidate_code: &str) -> Result<f32, StyleStoreError> {
        let profile = self.get_profile(scope)?;
        Ok(self.trainer.style_score(&profile, candidate_code))
    }
}

impl Default for StyleProfile {
    fn default() -> Self {
        Self {
            sample_count: 0,
            preferred_indent: "spaces:4".to_string(),
            max_line_length: 100,
            snake_case_ratio: 0.5,
            notes: vec!["No style data collected yet".to_string()],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StyleDatasetBuilder {
    samples: Vec<StyleSample>,
}

impl StyleDatasetBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_sample(&mut self, source: impl Into<String>, content: impl Into<String>) {
        self.samples.push(StyleSample {
            source: source.into(),
            content: content.into(),
        });
    }

    #[must_use]
    pub fn build(self) -> Vec<StyleSample> {
        self.samples
    }
}

#[derive(Debug, Clone, Default)]
pub struct StyleTrainer;

impl StyleTrainer {
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn train(&self, samples: &[StyleSample]) -> StyleProfile {
        if samples.is_empty() {
            return StyleProfile::default();
        }

        let mut tab_indented_lines = 0_usize;
        let mut space_indented_lines = 0_usize;
        let mut max_line_length = 80_usize;
        let mut snake_case_hits = 0_usize;
        let mut identifier_checks = 0_usize;

        for sample in samples {
            for line in sample.content.lines() {
                if line.starts_with('\t') {
                    tab_indented_lines += 1;
                } else if line.starts_with("    ") {
                    space_indented_lines += 1;
                }

                max_line_length = max_line_length.max(line.chars().count());

                for token in line.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
                    if token.len() < 3 {
                        continue;
                    }
                    identifier_checks += 1;
                    if looks_like_snake_case(token) {
                        snake_case_hits += 1;
                    }
                }
            }
        }

        let preferred_indent = if tab_indented_lines > space_indented_lines {
            "tabs".to_string()
        } else {
            "spaces:4".to_string()
        };

        let snake_case_ratio = if identifier_checks == 0 {
            0.0
        } else {
            snake_case_hits as f32 / identifier_checks as f32
        };

        StyleProfile {
            sample_count: samples.len(),
            preferred_indent,
            max_line_length,
            snake_case_ratio,
            notes: vec![
                "Generated from local repository style samples".to_string(),
                "Use with retrieval memory + prompt conditioning".to_string(),
            ],
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn style_score(&self, profile: &StyleProfile, candidate_code: &str) -> f32 {
        let mut score = 0.0_f32;
        let line_count = candidate_code.lines().count().max(1) as f32;

        let indent_hits = candidate_code
            .lines()
            .filter(|line| {
                if profile.preferred_indent == "tabs" {
                    line.starts_with('\t') || line.trim().is_empty()
                } else {
                    line.starts_with("    ") || line.trim().is_empty()
                }
            })
            .count() as f32;
        score += 0.4_f32 * (indent_hits / line_count);

        let within_length = candidate_code
            .lines()
            .filter(|line| line.chars().count() <= profile.max_line_length)
            .count() as f32;
        score += 0.3_f32 * (within_length / line_count);

        let mut snake_tokens = 0.0_f32;
        let mut total_tokens = 0.0_f32;
        for token in candidate_code.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
            if token.len() < 3 {
                continue;
            }
            total_tokens += 1.0;
            if looks_like_snake_case(token) {
                snake_tokens += 1.0;
            }
        }
        let target = profile.snake_case_ratio;
        let actual = if total_tokens == 0.0 {
            0.0
        } else {
            snake_tokens / total_tokens
        };
        score += 0.3_f32 * (1.0_f32 - (target - actual).abs()).clamp(0.0, 1.0);

        score.clamp(0.0, 1.0)
    }
}

fn looks_like_snake_case(token: &str) -> bool {
    token.contains('_')
        && token
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryStyleProfileStore, StyleDatasetBuilder, StyleSample, StyleScope, StyleTrainer,
        StyleTrainingService,
    };

    #[test]
    fn trainer_extracts_style_profile() {
        let mut builder = StyleDatasetBuilder::new();
        builder.add_sample(
            "src/lib.rs",
            "fn parse_args() {\n    let user_name = \"g\";\n}\n",
        );
        builder.add_sample(
            "src/main.rs",
            "fn build_graph() {\n    let node_count = 2;\n}\n",
        );

        let samples = builder.build();
        let trainer = StyleTrainer;
        let profile = trainer.train(&samples);

        assert_eq!(profile.sample_count, 2);
        assert_eq!(profile.preferred_indent, "spaces:4");
        assert!(profile.snake_case_ratio > 0.1);
    }

    #[test]
    fn style_score_prefers_profile_conformant_code() {
        let mut builder = StyleDatasetBuilder::new();
        builder.add_sample("seed", "fn alpha_beta() {\n    let delta_value = 10;\n}\n");
        let samples = builder.build();
        let trainer = StyleTrainer;
        let profile = trainer.train(&samples);

        let good = "fn alpha_beta() {\n    let delta_value = 10;\n}\n";
        let bad = "fn alphaBeta(){\n\tlet DeltaValue=10;\n}\n";

        assert!(trainer.style_score(&profile, good) > trainer.style_score(&profile, bad));
    }

    #[test]
    fn service_train_persists_profile_for_scope() {
        let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
        let scope = StyleScope::new("session-a", Some("repo-a".to_string()), None);
        let samples = vec![
            StyleSample {
                source: "a.rs".to_string(),
                content: "fn parse_args() {\n    let user_name = 1;\n}\n".to_string(),
            },
            StyleSample {
                source: "b.rs".to_string(),
                content: "fn build_graph() {\n    let node_count = 2;\n}\n".to_string(),
            },
        ];

        let trained = service
            .train(&scope, &samples)
            .expect("training should persist profile");
        let loaded = service
            .get_profile(&scope)
            .expect("profile should load for trained scope");

        assert_eq!(loaded, trained);
    }

    #[test]
    fn service_get_profile_defaults_when_missing() {
        let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
        let scope = StyleScope::new("session-missing", None, None);

        let profile = service
            .get_profile(&scope)
            .expect("missing profile should fall back to default");

        assert_eq!(profile.sample_count, 0);
        assert_eq!(profile.preferred_indent, "spaces:4");
    }

    #[test]
    fn service_score_uses_persisted_profile() {
        let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
        let scope = StyleScope::new("session-tabs", Some("repo-a".to_string()), None);
        let samples = vec![StyleSample {
            source: "tabs.rs".to_string(),
            content: "fn x() {\n\tlet snake_case = 1;\n}\n".to_string(),
        }];
        service
            .train(&scope, &samples)
            .expect("training should succeed");

        let tabs_candidate = "fn x() {\n\tlet snake_case = 1;\n}\n";
        let spaces_candidate = "fn x() {\n    let snake_case = 1;\n}\n";
        let tabs_score = service
            .score(&scope, tabs_candidate)
            .expect("score should load trained profile");
        let spaces_score = service
            .score(&scope, spaces_candidate)
            .expect("score should load trained profile");

        assert!(tabs_score > spaces_score);
    }

    #[test]
    fn service_scopes_do_not_leak() {
        let service = StyleTrainingService::new(InMemoryStyleProfileStore::new());
        let scope_a = StyleScope::new("session-a", Some("repo-a".to_string()), None);
        let scope_b = StyleScope::new("session-b", Some("repo-a".to_string()), None);
        let samples = vec![StyleSample {
            source: "seed.rs".to_string(),
            content: "fn alpha_beta() {\n    let delta_value = 10;\n}\n".to_string(),
        }];

        let trained = service
            .train(&scope_a, &samples)
            .expect("training should succeed for first scope");
        let missing = service
            .get_profile(&scope_b)
            .expect("other scope should not see profile");

        assert_eq!(trained.sample_count, 1);
        assert_eq!(missing.sample_count, 0);
        assert_ne!(trained.notes, missing.notes);
    }
}
