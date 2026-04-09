use reqwest::Client;

#[derive(Debug, Clone)]
pub struct LinearClientConfig {
    pub api_url: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct LinearIssueCommentRequest {
    pub issue_id: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct LinearClient {
    http: Client,
    config: LinearClientConfig,
}

impl LinearClient {
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("ORBIT_LINEAR_API_TOKEN").ok()?;
        let api_url = std::env::var("ORBIT_LINEAR_API_URL")
            .unwrap_or_else(|_| "https://api.linear.app/graphql".to_string());
        Some(Self {
            http: Client::new(),
            config: LinearClientConfig { api_url, token },
        })
    }

    pub async fn create_issue_comment(
        &self,
        request: LinearIssueCommentRequest,
    ) -> Result<(), String> {
        // Minimal GraphQL mutation for creating a comment on an issue
        #[derive(serde::Serialize)]
        struct Variables<'a> {
            input: CommentInput<'a>,
        }

        #[derive(serde::Serialize)]
        struct CommentInput<'a> {
            #[serde(rename = "issueId")]
            issue_id: &'a str,
            body: &'a str,
        }

        let payload = serde_json::json!({
            "query": "mutation CommentCreate($input: CommentCreateInput!) { commentCreate(input: $input) { success } }",
            "variables": Variables {
                input: CommentInput {
                    issue_id: &request.issue_id,
                    body: &request.body,
                }
            }
        });

        let response = self
            .http
            .post(&self.config.api_url)
            .bearer_auth(&self.config.token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("linear request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("linear returned status {}", response.status()));
        }

        Ok(())
    }
}
