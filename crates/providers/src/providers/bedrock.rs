use super::openai::{AuthScheme, OpenAiCompatConfig};

pub const DEFAULT_BASE_URL: &str = "https://bedrock-runtime.us-east-1.amazonaws.com/openai/v1";
const BEDROCK_ENV_VARS: &[&str] = &["BEDROCK_API_KEY"];

#[must_use]
pub const fn config() -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider_name: "AWS Bedrock",
        api_key_env: "BEDROCK_API_KEY",
        credential_env_vars: BEDROCK_ENV_VARS,
        base_url_env: "BEDROCK_BASE_URL",
        default_base_url: DEFAULT_BASE_URL,
        auth_scheme: AuthScheme::Bearer,
    }
}
