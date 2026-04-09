use super::openai::{AuthScheme, OpenAiCompatConfig};

pub const DEFAULT_BASE_URL: &str =
    "https://YOUR_RESOURCE_NAME.openai.azure.com/openai/deployments/YOUR_DEPLOYMENT";
const AZURE_ENV_VARS: &[&str] = &["AZURE_OPENAI_API_KEY"];

#[must_use]
pub const fn config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "Microsoft Azure",
        api_key_env: "AZURE_OPENAI_API_KEY",
        credential_env_vars: AZURE_ENV_VARS,
        base_url_env: "AZURE_OPENAI_BASE_URL",
        default_base_url: DEFAULT_BASE_URL,
        auth_scheme: AuthScheme::ApiKeyHeader,
    }
}
