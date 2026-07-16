use std::time::{SystemTime, UNIX_EPOCH};

use orbit_runtime::{load_oauth_credentials_for, save_oauth_credentials_for, OAuthTokenSet};
use reqwest::Client;
use serde::Deserialize;

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
    oauth_token_set: Option<OAuthTokenSet>,
}

#[derive(Debug, Deserialize)]
struct LinearTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

fn now_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn token_is_expired(token_set: &OAuthTokenSet) -> bool {
    token_set
        .expires_at
        .is_some_and(|expires_at| expires_at <= now_unix_timestamp())
}

impl LinearClient {
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("ORBIT_LINEAR_API_TOKEN").ok()?;
        let api_url = std::env::var("ORBIT_LINEAR_API_URL")
            .unwrap_or_else(|_| "https://api.linear.app/graphql".to_string());
        Some(Self {
            http: Client::new(),
            config: LinearClientConfig { api_url, token },
            oauth_token_set: None,
        })
    }

    pub async fn from_oauth_or_env() -> Option<Self> {
        let api_url = std::env::var("ORBIT_LINEAR_API_URL")
            .unwrap_or_else(|_| "https://api.linear.app/graphql".to_string());

        if let Ok(Some(token_set)) = load_oauth_credentials_for("linear") {
            if !token_is_expired(&token_set) {
                return Some(Self {
                    http: Client::new(),
                    config: LinearClientConfig {
                        api_url,
                        token: token_set.access_token.clone(),
                    },
                    oauth_token_set: Some(token_set),
                });
            }

            if let Some(refresh_token) = token_set.refresh_token.clone() {
                if let Ok(Some(refreshed)) = Self::refresh_token(&refresh_token).await {
                    let _ = save_oauth_credentials_for(&refreshed, "linear");
                    return Some(Self {
                        http: Client::new(),
                        config: LinearClientConfig {
                            api_url,
                            token: refreshed.access_token.clone(),
                        },
                        oauth_token_set: Some(refreshed),
                    });
                }
            }
        }

        Self::from_env()
    }

    async fn refresh_token(refresh_token: &str) -> Result<Option<OAuthTokenSet>, String> {
        let client_id = std::env::var("ORBIT_LINEAR_CLIENT_ID").ok();
        let client_secret = std::env::var("ORBIT_LINEAR_CLIENT_SECRET").ok();

        let mut params: Vec<(&str, &str)> = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        let client_id_str;
        let client_secret_str;
        if let Some(ref id) = client_id {
            client_id_str = id.clone();
            params.push(("client_id", &client_id_str));
        }
        if let Some(ref secret) = client_secret {
            client_secret_str = secret.clone();
            params.push(("client_secret", &client_secret_str));
        }

        let http = Client::new();
        let response = http
            .post("https://api.linear.app/oauth/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("token refresh failed: {e}"))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let token_response: LinearTokenResponse = response
            .json()
            .await
            .map_err(|e| format!("failed to parse refresh response: {e}"))?;

        let expires_at = token_response
            .expires_in
            .map(|secs| now_unix_timestamp() + secs);

        Ok(Some(OAuthTokenSet {
            access_token: token_response.access_token,
            refresh_token: token_response
                .refresh_token
                .or(Some(refresh_token.to_string())),
            expires_at,
            scopes: token_response
                .scope
                .map(|s| {
                    s.split_whitespace()
                        .map(str::trim)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
        }))
    }

    pub async fn create_issue_comment(
        &self,
        request: LinearIssueCommentRequest,
    ) -> Result<(), String> {
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
