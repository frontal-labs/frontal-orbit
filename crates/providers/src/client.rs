use crate::error::ApiError;
use crate::prompt_cache::{PromptCache, PromptCacheRecord, PromptCacheStats};
use crate::providers::anthropic::{self, AnthropicClient, AuthSource};
use crate::providers::azure;
use crate::providers::bedrock;
use crate::providers::frontal;
use crate::providers::ollama;
use crate::providers::openai::{self, OpenAiCompatClient};
use crate::providers::xai;
use crate::providers::{Provider, ProviderKind};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};
use orbit_telemetry::SessionTracer;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ProviderClient {
    Anthropic(AnthropicClient),
    Xai(OpenAiCompatClient),
    OpenAi(OpenAiCompatClient),
    Frontal(OpenAiCompatClient),
    Bedrock(OpenAiCompatClient),
    Azure(OpenAiCompatClient),
    Ollama(ollama::OllamaClient),
}

impl ProviderClient {
    pub fn from_model(model: &str) -> Result<Self, ApiError> {
        Self::from_model_with_anthropic_auth(model, None)
    }

    pub fn from_model_with_anthropic_auth(
        model: &str,
        anthropic_auth: Option<AuthSource>,
    ) -> Result<Self, ApiError> {
        let resolved_model = crate::providers::resolve_model_alias(model);
        match crate::providers::detect_provider_kind(&resolved_model) {
            crate::providers::ProviderKind::Anthropic => {
                Ok(Self::Anthropic(match anthropic_auth {
                    Some(auth) => AnthropicClient::from_auth(auth),
                    None => AnthropicClient::from_auth(
                        crate::providers::anthropic::AuthSource::from_env()?,
                    ),
                }))
            }
            crate::providers::ProviderKind::OpenAi => Ok(Self::OpenAi(
                OpenAiCompatClient::from_env(openai::config())?,
            )),
            crate::providers::ProviderKind::Xai => {
                Ok(Self::Xai(OpenAiCompatClient::from_env(xai::config())?))
            }
            crate::providers::ProviderKind::Frontal => Ok(Self::Frontal(
                OpenAiCompatClient::from_env(frontal::config())?,
            )),
            crate::providers::ProviderKind::Bedrock => Ok(Self::Bedrock(
                OpenAiCompatClient::from_env(bedrock::config())?,
            )),
            crate::providers::ProviderKind::Azure => {
                Ok(Self::Azure(OpenAiCompatClient::from_env(azure::config())?))
            }
            crate::providers::ProviderKind::Ollama => Ok(Self::Ollama(ollama::OllamaClient::new(
                std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
                resolved_model,
            ))),
        }
    }

    #[must_use]
    pub const fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::Anthropic(_) => ProviderKind::Anthropic,
            Self::Xai(_) => ProviderKind::Xai,
            Self::OpenAi(_) => ProviderKind::OpenAi,
            Self::Frontal(_) => ProviderKind::Frontal,
            Self::Bedrock(_) => ProviderKind::Bedrock,
            Self::Azure(_) => ProviderKind::Azure,
            Self::Ollama(_) => ProviderKind::Ollama,
        }
    }

    #[must_use]
    pub fn with_prompt_cache(self, prompt_cache: PromptCache) -> Self {
        match self {
            Self::Anthropic(client) => Self::Anthropic(client.with_prompt_cache(prompt_cache)),
            other => other,
        }
    }

    #[must_use]
    pub fn with_session_tracer(self, session_tracer: SessionTracer) -> Self {
        match self {
            Self::Anthropic(client) => Self::Anthropic(client.with_session_tracer(session_tracer)),
            other => other,
        }
    }

    #[must_use]
    pub fn prompt_cache_stats(&self) -> Option<PromptCacheStats> {
        match self {
            Self::Anthropic(client) => client.prompt_cache_stats(),
            Self::Xai(_)
            | Self::OpenAi(_)
            | Self::Frontal(_)
            | Self::Bedrock(_)
            | Self::Azure(_)
            | Self::Ollama(_) => None,
        }
    }

    #[must_use]
    pub fn take_last_prompt_cache_record(&self) -> Option<PromptCacheRecord> {
        match self {
            Self::Anthropic(client) => client.take_last_prompt_cache_record(),
            Self::Xai(_)
            | Self::OpenAi(_)
            | Self::Frontal(_)
            | Self::Bedrock(_)
            | Self::Azure(_)
            | Self::Ollama(_) => None,
        }
    }

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        match self {
            Self::Anthropic(client) => client.send_message(request).await,
            Self::Xai(client)
            | Self::OpenAi(client)
            | Self::Frontal(client)
            | Self::Bedrock(client)
            | Self::Azure(client) => client.send_message(request).await,
            Self::Ollama(client) => client.send_message(request).await,
        }
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        match self {
            Self::Anthropic(client) => client
                .stream_message(request)
                .await
                .map(MessageStream::Anthropic),
            Self::Xai(client)
            | Self::OpenAi(client)
            | Self::Frontal(client)
            | Self::Bedrock(client)
            | Self::Azure(client) => client
                .stream_message(request)
                .await
                .map(MessageStream::OpenAiCompat),
            Self::Ollama(client) => client
                .stream_message(request)
                .await
                .map(MessageStream::Ollama),
        }
    }
}

#[derive(Debug)]
pub enum MessageStream {
    Anthropic(anthropic::MessageStream),
    OpenAiCompat(openai::MessageStream),
    Ollama(ollama::MessageStream),
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Anthropic(stream) => stream.request_id(),
            Self::OpenAiCompat(stream) => stream.request_id(),
            Self::Ollama(stream) => stream.request_id(),
        }
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        match self {
            Self::Anthropic(stream) => stream.next_event().await,
            Self::OpenAiCompat(stream) => stream.next_event().await,
            Self::Ollama(stream) => stream.next_event().await,
        }
    }
}

pub use anthropic::{
    oauth_token_is_expired, resolve_saved_oauth_token, resolve_startup_auth_source, OAuthTokenSet,
};
#[must_use]
pub fn read_base_url() -> String {
    anthropic::read_base_url()
}

#[must_use]
pub fn read_xai_base_url() -> String {
    openai::read_base_url(xai::config())
}

#[must_use]
pub fn read_frontal_base_url() -> String {
    openai::read_base_url(frontal::config())
}

#[must_use]
pub fn read_bedrock_base_url() -> String {
    openai::read_base_url(bedrock::config())
}

#[must_use]
pub fn read_azure_base_url() -> String {
    openai::read_base_url(azure::config())
}

#[cfg(test)]
mod tests {
    use crate::providers::{detect_provider_kind, resolve_model_alias, ProviderKind};

    #[test]
    fn resolves_existing_and_grok_aliases() {
        assert_eq!(resolve_model_alias("opus"), "claude-opus-4-6");
        assert_eq!(resolve_model_alias("grok"), "grok-3");
        assert_eq!(resolve_model_alias("grok-mini"), "grok-3-mini");
        assert_eq!(resolve_model_alias("frontal"), "frontal");
        assert_eq!(resolve_model_alias("bedrock"), "bedrock");
        assert_eq!(resolve_model_alias("azure"), "azure");
    }

    #[test]
    fn provider_detection_prefers_model_family() {
        assert_eq!(detect_provider_kind("grok-3"), ProviderKind::Xai);
        assert_eq!(
            detect_provider_kind("frontal/gpt-4o-mini"),
            ProviderKind::Frontal
        );
        assert_eq!(
            detect_provider_kind("bedrock/meta.llama3-70b-instruct-v1:0"),
            ProviderKind::Bedrock
        );
        assert_eq!(detect_provider_kind("azure/gpt-4o"), ProviderKind::Azure);
        assert_eq!(
            detect_provider_kind("claude-sonnet-4-6"),
            ProviderKind::Anthropic
        );
    }
}
