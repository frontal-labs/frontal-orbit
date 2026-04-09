use reqwest::Client;

#[derive(Debug, Clone)]
pub struct GraphiteClientConfig {
    pub api_url: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct GraphiteStackCommentRequest {
    pub stack_id: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct GraphiteClient {
    http: Client,
    config: GraphiteClientConfig,
}

impl GraphiteClient {
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("ORBIT_GRAPHITE_API_TOKEN").ok()?;
        let api_url = std::env::var("ORBIT_GRAPHITE_API_URL")
            .unwrap_or_else(|_| "https://graphite.dev/api".to_string());
        Some(Self {
            http: Client::new(),
            config: GraphiteClientConfig { api_url, token },
        })
    }

    pub async fn create_stack_comment(
        &self,
        request: GraphiteStackCommentRequest,
    ) -> Result<(), String> {
        let url = format!(
            "{}/stacks/{}/comments",
            self.config.api_url, request.stack_id
        );
        let payload = serde_json::json!({ "message": request.body });

        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("graphite request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("graphite returned status {}", response.status()));
        }

        Ok(())
    }
}
