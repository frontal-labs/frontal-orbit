use super::openai::{AuthScheme, OpenAiCompatConfig};

/// `Azure OpenAI` endpoints are per-resource, so there is no usable shared
/// default. This template is documentation only: `requires_base_url` makes
/// client construction fail with a named-variable error when
/// `AZURE_OPENAI_BASE_URL` is unset, rather than letting the template reach DNS.
pub const DEFAULT_BASE_URL: &str =
    "https://<resource>.openai.azure.com/openai/deployments/<deployment>";
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
        requires_base_url: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{config, DEFAULT_BASE_URL};

    /// Azure endpoints are per-resource. Selecting Azure without
    /// `AZURE_OPENAI_BASE_URL` must fail with a message naming that variable
    /// rather than letting a template hostname reach DNS.
    #[test]
    fn requires_an_explicit_base_url() {
        assert!(config().requires_base_url);
        assert_eq!(config().base_url_env, "AZURE_OPENAI_BASE_URL");
    }

    /// Keep the documented default obviously unusable, so it can never be
    /// mistaken for a real endpoint.
    #[test]
    fn default_base_url_is_a_visible_template() {
        assert!(DEFAULT_BASE_URL.contains("<resource>"));
        assert!(DEFAULT_BASE_URL.contains("<deployment>"));
    }
}
