use super::openai::{AuthScheme, OpenAiCompatConfig};

pub const DEFAULT_BASE_URL: &str = "https://api.frontal.ai/v1";
const FRONTAL_ENV_VARS: &[&str] = &["FRONTAL_API_KEY"];

#[must_use]
pub const fn config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "Frontal",
        api_key_env: "FRONTAL_API_KEY",
        credential_env_vars: FRONTAL_ENV_VARS,
        base_url_env: "FRONTAL_BASE_URL",
        default_base_url: DEFAULT_BASE_URL,
        auth_scheme: AuthScheme::Bearer,
    }
}
