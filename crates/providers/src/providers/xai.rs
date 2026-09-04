use super::openai::{AuthScheme, OpenAiCompatConfig};

pub const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";
const XAI_ENV_VARS: &[&str] = &["XAI_API_KEY"];

#[must_use]
pub const fn config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "xAI",
        api_key_env: "XAI_API_KEY",
        credential_env_vars: XAI_ENV_VARS,
        base_url_env: "XAI_BASE_URL",
        default_base_url: DEFAULT_BASE_URL,
        auth_scheme: AuthScheme::Bearer,
        requires_base_url: false,
    }
}
