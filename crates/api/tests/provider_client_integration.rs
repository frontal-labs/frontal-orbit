use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

use orbit_api::{
    read_azure_base_url, read_bedrock_base_url, read_frontal_base_url, read_xai_base_url, ApiError,
    AuthSource, ProviderClient, ProviderKind,
};

#[test]
fn provider_client_routes_grok_aliases_through_xai() {
    let _lock = env_lock();
    let _xai_api_key = EnvVarGuard::set("XAI_API_KEY", Some("xai-test-key"));

    let client = ProviderClient::from_model("grok-mini").expect("grok alias should resolve");

    assert_eq!(client.provider_kind(), ProviderKind::Xai);
}

#[test]
fn provider_client_reports_missing_xai_credentials_for_grok_models() {
    let _lock = env_lock();
    let _xai_api_key = EnvVarGuard::set("XAI_API_KEY", None);

    let error = ProviderClient::from_model("grok-3")
        .expect_err("grok requests without XAI_API_KEY should fail fast");

    match error {
        ApiError::MissingCredentials { provider, env_vars } => {
            assert_eq!(provider, "xAI");
            assert_eq!(env_vars, &["XAI_API_KEY"]);
        }
        other => panic!("expected missing xAI credentials, got {other:?}"),
    }
}

#[test]
fn provider_client_uses_explicit_anthropic_auth_without_env_lookup() {
    let _lock = env_lock();
    let _anthropic_api_key = EnvVarGuard::set("ANTHROPIC_API_KEY", None);
    let _anthropic_auth_token = EnvVarGuard::set("ANTHROPIC_AUTH_TOKEN", None);

    let client = ProviderClient::from_model_with_anthropic_auth(
        "claude-sonnet-4-6",
        Some(AuthSource::ApiKey("anthropic-test-key".to_string())),
    )
    .expect("explicit anthropic auth should avoid env lookup");

    assert_eq!(client.provider_kind(), ProviderKind::Anthropic);
}

#[test]
fn read_xai_base_url_prefers_env_override() {
    let _lock = env_lock();
    let _xai_base_url = EnvVarGuard::set("XAI_BASE_URL", Some("https://example.xai.test/v1"));

    assert_eq!(read_xai_base_url(), "https://example.xai.test/v1");
}

#[test]
fn provider_client_routes_bedrock_models_and_requires_credentials() {
    let _lock = env_lock();
    let _bedrock_api_key = EnvVarGuard::set("BEDROCK_API_KEY", None);

    let error = ProviderClient::from_model("bedrock/meta.llama3-70b-instruct-v1:0")
        .expect_err("bedrock models without BEDROCK_API_KEY should fail fast");

    match error {
        ApiError::MissingCredentials { provider, env_vars } => {
            assert_eq!(provider, "AWS Bedrock");
            assert_eq!(env_vars, &["BEDROCK_API_KEY"]);
        }
        other => panic!("expected missing Bedrock credentials, got {other:?}"),
    }
}

#[test]
fn provider_client_routes_azure_models_and_requires_credentials() {
    let _lock = env_lock();
    let _azure_api_key = EnvVarGuard::set("AZURE_OPENAI_API_KEY", None);

    let error = ProviderClient::from_model("azure/gpt-4.1")
        .expect_err("azure models without AZURE_OPENAI_API_KEY should fail fast");

    match error {
        ApiError::MissingCredentials { provider, env_vars } => {
            assert_eq!(provider, "Microsoft Azure");
            assert_eq!(env_vars, &["AZURE_OPENAI_API_KEY"]);
        }
        other => panic!("expected missing Azure credentials, got {other:?}"),
    }
}

#[test]
fn provider_client_routes_frontal_models_and_requires_credentials() {
    let _lock = env_lock();
    let _frontal_api_key = EnvVarGuard::set("FRONTAL_API_KEY", None);

    let error = ProviderClient::from_model("frontal/gpt-4.1")
        .expect_err("frontal models without FRONTAL_API_KEY should fail fast");

    match error {
        ApiError::MissingCredentials { provider, env_vars } => {
            assert_eq!(provider, "Frontal");
            assert_eq!(env_vars, &["FRONTAL_API_KEY"]);
        }
        other => panic!("expected missing Frontal credentials, got {other:?}"),
    }
}

#[test]
fn bedrock_and_azure_base_urls_prefer_env_overrides() {
    let _lock = env_lock();
    let _bedrock_base_url = EnvVarGuard::set(
        "BEDROCK_BASE_URL",
        Some("https://example.bedrock.test/openai/v1"),
    );
    let _azure_base_url = EnvVarGuard::set(
        "AZURE_OPENAI_BASE_URL",
        Some("https://example.azure.test/openai/deployments/demo"),
    );

    assert_eq!(
        read_bedrock_base_url(),
        "https://example.bedrock.test/openai/v1"
    );
    assert_eq!(
        read_azure_base_url(),
        "https://example.azure.test/openai/deployments/demo"
    );
}

#[test]
fn frontal_base_url_prefers_env_override() {
    let _lock = env_lock();
    let _frontal_base_url =
        EnvVarGuard::set("FRONTAL_BASE_URL", Some("https://api.frontal.test/v1"));

    assert_eq!(read_frontal_base_url(), "https://api.frontal.test/v1");
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let original = std::env::var_os(key);
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
