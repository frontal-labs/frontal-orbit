use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::providers::{Provider, ProviderFuture};
use crate::sse::SseParser;
use crate::types::{
    InputContentBlock, MessageRequest, MessageResponse, MessageStopEvent, OutputContentBlock,
    StreamEvent, Usage,
};

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug)]
pub struct MessageStream {
    request_id: Option<String>,
    finished: bool,
    _response: reqwest::Response,
    _parser: SseParser,
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        if self.finished {
            return Ok(None);
        }
        self.finished = true;
        Ok(Some(StreamEvent::MessageStop(MessageStopEvent {})))
    }
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> Self {
        let client = Client::new();
        Self {
            client,
            base_url,
            model,
        }
    }

    pub fn from_env() -> Result<Self, ApiError> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama2".to_string());
        Ok(Self::new(base_url, model))
    }
}

impl Provider for OllamaClient {
    type Stream = MessageStream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse> {
        Box::pin(async move {
            let ollama_request = OllamaRequest {
                model: self.model.clone(),
                messages: request
                    .messages
                    .iter()
                    .map(|msg| OllamaMessage {
                        role: msg.role.clone(),
                        content: msg
                            .content
                            .iter()
                            .map(|block| match block {
                                InputContentBlock::Text { text } => text.clone(),
                                _ => format!("{:?}", block),
                            })
                            .collect(),
                    })
                    .collect(),
                stream: false,
            };

            let response = self
                .client
                .post(format!("{}/api/chat", self.base_url))
                .json(&ollama_request)
                .send()
                .await
                .map_err(|e| ApiError::Http(e))?;
            let status = response.status();
            let body = response.text().await.map_err(ApiError::Http)?;
            if !status.is_success() {
                return Err(ApiError::Api {
                    status,
                    error_type: Some("ollama_error".to_string()),
                    message: None,
                    request_id: None,
                    body,
                    retryable: false,
                });
            }

            let ollama_response: OllamaResponse = match serde_json::from_str(&body) {
                Ok(parsed) => parsed,
                Err(err) => {
                    // Some Ollama builds still return newline-delimited JSON chunks for chat responses.
                    let maybe_fallback = body.lines().rev().find(|line| !line.trim().is_empty());
                    let Some(fallback) = maybe_fallback else {
                        return Err(ApiError::Json(err));
                    };
                    serde_json::from_str(fallback).map_err(|_| ApiError::Json(err))?
                }
            };

            Ok(MessageResponse {
                id: "default".to_string(),
                kind: "message".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::Text {
                    text: ollama_response.message.content,
                }],
                model: self.model.clone(),
                stop_reason: None,
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 0,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                    output_tokens: 0,
                },
                request_id: None,
            })
        })
    }

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream> {
        Box::pin(async move {
            // Create a simple streaming response by reusing the HTTP response from send_message
            let stream_request = OllamaRequest {
                model: self.model.clone(),
                messages: request
                    .messages
                    .iter()
                    .map(|msg| OllamaMessage {
                        role: msg.role.clone(),
                        content: msg
                            .content
                            .iter()
                            .map(|block| match block {
                                InputContentBlock::Text { text } => text.clone(),
                                _ => format!("{:?}", block),
                            })
                            .collect(),
                    })
                    .collect(),
                stream: true,
            };

            let stream_response = self
                .client
                .post(format!("{}/api/chat", self.base_url))
                .json(&stream_request)
                .send()
                .await
                .map_err(|e| ApiError::Http(e))?;

            Ok(MessageStream {
                request_id: Some("default".to_string()),
                finished: false,
                _response: stream_response,
                _parser: SseParser::new(),
            })
        })
    }
}
