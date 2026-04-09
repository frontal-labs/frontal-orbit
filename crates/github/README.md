# Orbit GitHub

GitHub API client crate for Orbit, providing comprehensive GitHub integration for pull requests, issues, and check runs.

## Overview

The `orbit-github` crate provides a clean, type-safe interface to the GitHub REST API focused on the operations most commonly needed by AI agent workflows. It handles authentication, request validation, and response parsing while providing clear error handling.

## Features

- **Pull Request Management**: Create and manage GitHub pull requests
- **Issue Comments**: Add comments to GitHub issues and pull requests  
- **Check Runs**: Create GitHub check runs for CI/CD integration
- **Repository Parsing**: Parse GitHub repository URLs from various formats
- **Type Safety**: Strongly typed request/response structures
- **Error Handling**: Comprehensive error types with detailed messages

## Key Components

### GitHubClient
Main client for GitHub API operations with methods for:
- `create_pull_request()` - Create new pull requests
- `create_issue_comment()` - Add comments to issues/PRs
- `create_check_run()` - Create check runs for commits

### Data Structures
- `GitHubRepoRef` - Repository reference (owner/repo)
- `GitHubPullRequestDraft` - Pull request creation payload
- `GitHubIssueCommentDraft` - Issue comment payload
- `GitHubCheckRunDraft` - Check run creation payload

### URL Parsing
- `parse_github_repo_url()` - Parse GitHub URLs from HTTPS, SSH, and HTTP formats

## Usage

```rust
use orbit_github::{GitHubClient, GitHubClientConfig, GitHubRepoRef, GitHubPullRequestDraft};

let client = GitHubClient::new(GitHubClientConfig {
    api_base: "https://api.github.com".to_string(),
    token: "your-github-token".to_string(),
});

let repo = GitHubRepoRef {
    owner: "owner".to_string(),
    repo: "repo".to_string(),
};

let pr_draft = GitHubPullRequestDraft {
    title: "Add new feature".to_string(),
    body: "This adds a new feature to the codebase.".to_string(),
    head: "feature-branch".to_string(),
    base: "main".to_string(),
    draft: false,
};

let pr = client.create_pull_request(&repo, &pr_draft)?;
```

## Authentication

The client uses a GitHub personal access token passed via the `GitHubClientConfig`. The token should have appropriate permissions for the operations you intend to perform.

## URL Formats Supported

- HTTPS: `https://github.com/owner/repo.git`
- HTTP: `http://github.com/owner/repo.git`
- SSH: `git@github.com:owner/repo.git`

## Dependencies

- `reqwest` - HTTP client with blocking support
- `serde` - Serialization/deserialization
- `serde_json` - JSON handling

## Error Handling

The crate provides detailed error types:
- `InvalidRepoUrl` - Malformed GitHub repository URLs
- `InvalidRequest` - Validation errors for request payloads
- `Http` - Network or HTTP response errors

## Testing

The crate includes comprehensive unit tests covering:
- URL parsing for various GitHub URL formats
- Request validation
- Endpoint URL construction
- JSON payload generation

Run tests with:
```bash
cargo test -p orbit-github
```
