use orbit_telemetry::{
    AnthropicRequestProfile, ClientIdentity, DEFAULT_AGENTIC_BETA, DEFAULT_ORBIT_VERSION,
    DEFAULT_PROMPT_CACHING_SCOPE_BETA,
};
use serde_json::{json, Map};

#[test]
fn new_creates_profile_with_default_betas() {
    let identity = ClientIdentity::new("app", "1.0");
    let profile = AnthropicRequestProfile::new(identity);
    assert_eq!(profile.anthropic_version, DEFAULT_ORBIT_VERSION);
    assert_eq!(profile.betas.len(), 2);
    assert!(profile.betas.contains(&DEFAULT_AGENTIC_BETA.to_string()));
    assert!(profile
        .betas
        .contains(&DEFAULT_PROMPT_CACHING_SCOPE_BETA.to_string()));
}

#[test]
fn with_beta_adds_new_beta() {
    let profile = AnthropicRequestProfile::default().with_beta("tools-2026-04-01");
    assert!(profile.betas.contains(&"tools-2026-04-01".to_string()));
}

#[test]
fn with_beta_does_not_add_duplicate() {
    let profile = AnthropicRequestProfile::default()
        .with_beta(DEFAULT_AGENTIC_BETA)
        .with_beta(DEFAULT_AGENTIC_BETA);
    assert_eq!(
        profile
            .betas
            .iter()
            .filter(|b| *b == DEFAULT_AGENTIC_BETA)
            .count(),
        1
    );
}

#[test]
fn with_extra_body_adds_field() {
    let profile =
        AnthropicRequestProfile::default().with_extra_body("metadata", json!({"key": "value"}));
    assert_eq!(
        profile.extra_body.get("metadata"),
        Some(&json!({"key": "value"}))
    );
}

#[test]
fn header_pairs_includes_version_and_user_agent() {
    let profile = AnthropicRequestProfile::new(ClientIdentity::new("cli", "2.0"));
    let headers = profile.header_pairs();
    assert!(headers.contains(&(
        "anthropic-version".to_string(),
        DEFAULT_ORBIT_VERSION.to_string()
    )));
    assert!(headers.contains(&("user-agent".to_string(), "cli/2.0".to_string())));
}

#[test]
fn header_pairs_includes_beta_header_when_betas_present() {
    let profile = AnthropicRequestProfile::default().with_beta("tools-2026-04-01");
    let headers = profile.header_pairs();
    let beta_header = headers
        .iter()
        .find(|(k, _)| k == "anthropic-beta")
        .map(|(_, v)| v);
    assert!(beta_header.is_some());
    let value = beta_header.unwrap();
    assert!(value.contains(DEFAULT_AGENTIC_BETA));
    assert!(value.contains("tools-2026-04-01"));
}

#[test]
fn header_pairs_empty_betas_omits_beta_header() {
    let profile = AnthropicRequestProfile {
        anthropic_version: DEFAULT_ORBIT_VERSION.to_string(),
        client_identity: ClientIdentity::default(),
        betas: vec![],
        extra_body: Map::default(),
    };
    let headers = profile.header_pairs();
    assert!(!headers.iter().any(|(k, _)| k == "anthropic-beta"));
}

#[test]
fn render_json_body_merges_betas_and_extra_body() {
    let profile = AnthropicRequestProfile::default().with_extra_body("custom", json!("val"));
    let body = profile
        .render_json_body(&json!({"model": "claude-sonnet"}))
        .expect("body should render");
    assert_eq!(body["model"], json!("claude-sonnet"));
    assert_eq!(body["custom"], json!("val"));
    assert!(body["betas"].is_array());
}

#[test]
fn render_json_body_without_betas() {
    let profile = AnthropicRequestProfile {
        anthropic_version: DEFAULT_ORBIT_VERSION.to_string(),
        client_identity: ClientIdentity::default(),
        betas: vec![],
        extra_body: Map::default(),
    };
    let body = profile
        .render_json_body(&json!({"model": "claude-sonnet"}))
        .expect("body should render");
    assert!(body.get("betas").is_none());
}

#[test]
fn render_json_body_fails_on_non_object() {
    let profile = AnthropicRequestProfile::default();
    let result = profile.render_json_body(&json!("string"));
    assert!(result.is_err());
}

#[test]
fn default_uses_default_client_identity() {
    let profile = AnthropicRequestProfile::default();
    assert_eq!(profile.client_identity, ClientIdentity::default());
    assert_eq!(profile.anthropic_version, DEFAULT_ORBIT_VERSION);
}

#[test]
fn clone_and_debug() {
    let profile = AnthropicRequestProfile::default();
    let cloned = profile.clone();
    assert_eq!(profile, cloned);
}
