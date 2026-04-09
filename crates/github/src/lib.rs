use std::error::Error;
use std::fmt;

use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepoRef {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubPullRequestDraft {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub number: u64,
    pub html_url: String,
    pub api_url: String,
    pub state: String,
    pub head_ref: String,
    pub base_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubIssueCommentDraft {
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubIssueComment {
    pub id: u64,
    pub html_url: String,
    pub api_url: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCheckRunOutput {
    pub title: String,
    pub summary: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCheckRunDraft {
    pub name: String,
    pub head_sha: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub details_url: Option<String>,
    pub output: Option<GitHubCheckRunOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCheckRun {
    pub id: u64,
    pub html_url: Option<String>,
    pub api_url: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub head_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubClientConfig {
    pub api_base: String,
    pub token: String,
}

#[derive(Debug)]
pub enum GitHubError {
    InvalidRepoUrl(String),
    InvalidRequest(String),
    Http(reqwest::Error),
}

impl fmt::Display for GitHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRepoUrl(url) => write!(f, "unsupported GitHub repository URL: {url}"),
            Self::InvalidRequest(message) => write!(f, "{message}"),
            Self::Http(error) => write!(f, "github request failed: {error}"),
        }
    }
}

impl Error for GitHubError {}

impl From<reqwest::Error> for GitHubError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

#[derive(Debug, Clone)]
pub struct GitHubClient {
    api_base: String,
    token: String,
    http: HttpClient,
}

impl GitHubClient {
    #[must_use]
    pub fn new(config: GitHubClientConfig) -> Self {
        Self {
            api_base: config.api_base.trim_end_matches('/').to_string(),
            token: config.token,
            http: HttpClient::new(),
        }
    }

    pub fn create_pull_request(
        &self,
        repo: &GitHubRepoRef,
        draft: &GitHubPullRequestDraft,
    ) -> Result<GitHubPullRequest, GitHubError> {
        validate_pull_request_draft(draft)?;
        let response = self
            .http
            .post(self.pull_request_endpoint(repo))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .json(&create_pull_request_body(draft))
            .send()?
            .error_for_status()?;
        let payload: GitHubPullRequestApiResponse = response.json()?;
        Ok(payload.into())
    }

    pub fn create_issue_comment(
        &self,
        repo: &GitHubRepoRef,
        issue_number: u64,
        draft: &GitHubIssueCommentDraft,
    ) -> Result<GitHubIssueComment, GitHubError> {
        validate_issue_comment_draft(draft)?;
        let response = self
            .http
            .post(self.issue_comment_endpoint(repo, issue_number))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .json(&create_issue_comment_body(draft))
            .send()?
            .error_for_status()?;
        let payload: GitHubIssueCommentApiResponse = response.json()?;
        Ok(payload.into())
    }

    pub fn create_check_run(
        &self,
        repo: &GitHubRepoRef,
        draft: &GitHubCheckRunDraft,
    ) -> Result<GitHubCheckRun, GitHubError> {
        validate_check_run_draft(draft)?;
        let response = self
            .http
            .post(self.check_runs_endpoint(repo))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .json(&create_check_run_body(draft))
            .send()?
            .error_for_status()?;
        let payload: GitHubCheckRunApiResponse = response.json()?;
        Ok(payload.into())
    }

    #[must_use]
    pub fn pull_request_endpoint(&self, repo: &GitHubRepoRef) -> String {
        format!("{}/repos/{}/{}/pulls", self.api_base, repo.owner, repo.repo)
    }

    #[must_use]
    pub fn issue_comment_endpoint(&self, repo: &GitHubRepoRef, issue_number: u64) -> String {
        format!(
            "{}/repos/{}/{}/issues/{issue_number}/comments",
            self.api_base, repo.owner, repo.repo
        )
    }

    #[must_use]
    pub fn check_runs_endpoint(&self, repo: &GitHubRepoRef) -> String {
        format!(
            "{}/repos/{}/{}/check-runs",
            self.api_base, repo.owner, repo.repo
        )
    }
}

pub fn parse_github_repo_url(url: &str) -> Result<GitHubRepoRef, GitHubError> {
    let trimmed = url.trim().trim_end_matches(".git").trim_end_matches('/');
    if let Some(path) = trimmed.strip_prefix("https://github.com/") {
        return parse_owner_repo(path, url);
    }
    if let Some(path) = trimmed.strip_prefix("http://github.com/") {
        return parse_owner_repo(path, url);
    }
    if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        return parse_owner_repo(path, url);
    }
    Err(GitHubError::InvalidRepoUrl(url.to_string()))
}

#[must_use]
pub fn create_pull_request_body(draft: &GitHubPullRequestDraft) -> Value {
    json!({
        "title": draft.title,
        "body": draft.body,
        "head": draft.head,
        "base": draft.base,
        "draft": draft.draft,
    })
}

#[must_use]
pub fn create_issue_comment_body(draft: &GitHubIssueCommentDraft) -> Value {
    json!({
        "body": draft.body,
    })
}

#[must_use]
pub fn create_check_run_body(draft: &GitHubCheckRunDraft) -> Value {
    let mut body = json!({
        "name": draft.name,
        "head_sha": draft.head_sha,
        "status": draft.status,
    });
    if let Some(conclusion) = &draft.conclusion {
        body["conclusion"] = json!(conclusion);
    }
    if let Some(details_url) = &draft.details_url {
        body["details_url"] = json!(details_url);
    }
    if let Some(output) = &draft.output {
        body["output"] = json!({
            "title": output.title,
            "summary": output.summary,
            "text": output.text,
        });
    }
    body
}

fn parse_owner_repo(path: &str, original_url: &str) -> Result<GitHubRepoRef, GitHubError> {
    let mut segments = path.split('/');
    let Some(owner) = segments.next().filter(|value| !value.is_empty()) else {
        return Err(GitHubError::InvalidRepoUrl(original_url.to_string()));
    };
    let Some(repo) = segments.next().filter(|value| !value.is_empty()) else {
        return Err(GitHubError::InvalidRepoUrl(original_url.to_string()));
    };
    if segments.next().is_some() {
        return Err(GitHubError::InvalidRepoUrl(original_url.to_string()));
    }
    Ok(GitHubRepoRef {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn validate_pull_request_draft(draft: &GitHubPullRequestDraft) -> Result<(), GitHubError> {
    if draft.title.trim().is_empty() {
        return Err(GitHubError::InvalidRequest(
            "pull request title must not be empty".to_string(),
        ));
    }
    if draft.head.trim().is_empty() || draft.base.trim().is_empty() {
        return Err(GitHubError::InvalidRequest(
            "pull request head and base must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_issue_comment_draft(draft: &GitHubIssueCommentDraft) -> Result<(), GitHubError> {
    if draft.body.trim().is_empty() {
        return Err(GitHubError::InvalidRequest(
            "issue comment body must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_check_run_draft(draft: &GitHubCheckRunDraft) -> Result<(), GitHubError> {
    if draft.name.trim().is_empty() {
        return Err(GitHubError::InvalidRequest(
            "check run name must not be empty".to_string(),
        ));
    }
    if draft.head_sha.trim().is_empty() {
        return Err(GitHubError::InvalidRequest(
            "check run head sha must not be empty".to_string(),
        ));
    }
    if draft.status.trim().is_empty() {
        return Err(GitHubError::InvalidRequest(
            "check run status must not be empty".to_string(),
        ));
    }
    if draft.status == "completed"
        && draft
            .conclusion
            .as_deref()
            .map(str::trim)
            .is_none_or(str::is_empty)
    {
        return Err(GitHubError::InvalidRequest(
            "completed check runs must include a conclusion".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubPullRequestApiRef {
    #[serde(rename = "ref")]
    git_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubPullRequestApiResponse {
    number: u64,
    html_url: String,
    url: String,
    state: String,
    head: GitHubPullRequestApiRef,
    base: GitHubPullRequestApiRef,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubIssueCommentApiResponse {
    id: u64,
    html_url: String,
    url: String,
    body: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubCheckRunApiResponse {
    id: u64,
    html_url: Option<String>,
    url: String,
    status: String,
    conclusion: Option<String>,
    head_sha: String,
}

impl From<GitHubPullRequestApiResponse> for GitHubPullRequest {
    fn from(value: GitHubPullRequestApiResponse) -> Self {
        Self {
            number: value.number,
            html_url: value.html_url,
            api_url: value.url,
            state: value.state,
            head_ref: value.head.git_ref,
            base_ref: value.base.git_ref,
        }
    }
}

impl From<GitHubIssueCommentApiResponse> for GitHubIssueComment {
    fn from(value: GitHubIssueCommentApiResponse) -> Self {
        Self {
            id: value.id,
            html_url: value.html_url,
            api_url: value.url,
            body: value.body,
        }
    }
}

impl From<GitHubCheckRunApiResponse> for GitHubCheckRun {
    fn from(value: GitHubCheckRunApiResponse) -> Self {
        Self {
            id: value.id,
            html_url: value.html_url,
            api_url: value.url,
            status: value.status,
            conclusion: value.conclusion,
            head_sha: value.head_sha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_and_ssh_repo_urls() {
        assert_eq!(
            parse_github_repo_url("https://github.com/acme/payments.git").unwrap(),
            GitHubRepoRef {
                owner: "acme".to_string(),
                repo: "payments".to_string(),
            }
        );
        assert_eq!(
            parse_github_repo_url("git@github.com:acme/payments.git").unwrap(),
            GitHubRepoRef {
                owner: "acme".to_string(),
                repo: "payments".to_string(),
            }
        );
    }

    #[test]
    fn rejects_non_github_repo_urls() {
        let error = parse_github_repo_url("https://gitlab.com/acme/payments.git")
            .expect_err("non-github URL should fail");
        assert!(error
            .to_string()
            .contains("unsupported GitHub repository URL"));
    }

    #[test]
    fn builds_pull_request_endpoint_and_body() {
        let client = GitHubClient::new(GitHubClientConfig {
            api_base: "https://api.github.com/".to_string(),
            token: "token".to_string(),
        });
        let repo = GitHubRepoRef {
            owner: "acme".to_string(),
            repo: "payments".to_string(),
        };
        let draft = GitHubPullRequestDraft {
            title: "Fix flaky release".to_string(),
            body: "This updates the release flow.".to_string(),
            head: "orbit/fix-release".to_string(),
            base: "main".to_string(),
            draft: true,
        };
        assert_eq!(
            client.pull_request_endpoint(&repo),
            "https://api.github.com/repos/acme/payments/pulls"
        );
        assert_eq!(
            create_pull_request_body(&draft),
            json!({
                "title": "Fix flaky release",
                "body": "This updates the release flow.",
                "head": "orbit/fix-release",
                "base": "main",
                "draft": true,
            })
        );
    }

    #[test]
    fn builds_issue_comment_and_check_run_endpoints_and_bodies() {
        let client = GitHubClient::new(GitHubClientConfig {
            api_base: "https://api.github.com/".to_string(),
            token: "token".to_string(),
        });
        let repo = GitHubRepoRef {
            owner: "acme".to_string(),
            repo: "payments".to_string(),
        };
        let comment = GitHubIssueCommentDraft {
            body: "Orbit completed the hosted task.".to_string(),
        };
        let check_run = GitHubCheckRunDraft {
            name: "orbit/hosted-task".to_string(),
            head_sha: "abc123def456".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            details_url: Some("http://orbit.test/v1/tasks/task_123".to_string()),
            output: Some(GitHubCheckRunOutput {
                title: "Hosted task completed".to_string(),
                summary: "Orbit finished the requested work.".to_string(),
                text: Some("All validations passed.".to_string()),
            }),
        };

        assert_eq!(
            client.issue_comment_endpoint(&repo, 42),
            "https://api.github.com/repos/acme/payments/issues/42/comments"
        );
        assert_eq!(
            create_issue_comment_body(&comment),
            json!({
                "body": "Orbit completed the hosted task.",
            })
        );
        assert_eq!(
            client.check_runs_endpoint(&repo),
            "https://api.github.com/repos/acme/payments/check-runs"
        );
        assert_eq!(
            create_check_run_body(&check_run),
            json!({
                "name": "orbit/hosted-task",
                "head_sha": "abc123def456",
                "status": "completed",
                "conclusion": "success",
                "details_url": "http://orbit.test/v1/tasks/task_123",
                "output": {
                    "title": "Hosted task completed",
                    "summary": "Orbit finished the requested work.",
                    "text": "All validations passed."
                }
            })
        );
    }

    #[test]
    fn validates_pull_request_draft() {
        let error = validate_pull_request_draft(&GitHubPullRequestDraft {
            title: "".to_string(),
            body: "".to_string(),
            head: "feature".to_string(),
            base: "main".to_string(),
            draft: false,
        })
        .expect_err("empty title should fail");
        assert!(error.to_string().contains("title"));
    }

    #[test]
    fn validates_issue_comment_and_check_run_drafts() {
        let comment_error = validate_issue_comment_draft(&GitHubIssueCommentDraft {
            body: "".to_string(),
        })
        .expect_err("empty comment body should fail");
        assert!(comment_error.to_string().contains("comment body"));

        let check_error = validate_check_run_draft(&GitHubCheckRunDraft {
            name: "orbit/hosted-task".to_string(),
            head_sha: "abc123def456".to_string(),
            status: "completed".to_string(),
            conclusion: None,
            details_url: None,
            output: None,
        })
        .expect_err("completed check run without conclusion should fail");
        assert!(check_error.to_string().contains("conclusion"));
    }
}
