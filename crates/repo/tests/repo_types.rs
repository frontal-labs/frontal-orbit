use orbit_repo::{
    PreparedRepo, RepoCheckoutRequest, RepoCommitRequest, RepoCommitResult, RepoPrepError,
    RepoSource, RepoStatus,
};

#[test]
fn repo_source_display_remote_url() {
    let source = RepoSource::RemoteUrl("https://github.com/acme/project.git".to_string());
    let display = source.display();
    assert!(display.contains("github.com"));
}

#[test]
fn repo_source_display_local_path() {
    let source = RepoSource::LocalPath("/tmp/repo".into());
    let display = source.display();
    assert!(display.contains("/tmp/repo"));
}

#[test]
fn repo_checkout_request_construction() {
    let request = RepoCheckoutRequest {
        workspace_root: "/tmp/workspaces".into(),
        checkout_id: "task-42".to_string(),
        source: RepoSource::RemoteUrl("https://github.com/acme/project.git".to_string()),
        repository: Some("acme/project".to_string()),
        base_ref: Some("main".to_string()),
        branch: Some("feature/new-feature".to_string()),
    };
    assert_eq!(request.checkout_id, "task-42");
    assert_eq!(request.repository.as_deref(), Some("acme/project"));
}

#[test]
fn repo_prep_error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let error = RepoPrepError::Io(io_err);
    let display = format!("{error}");
    assert!(display.contains("access denied"));
}

#[test]
fn repo_prep_error_display_invalid_request() {
    let error = RepoPrepError::InvalidRequest("checkout_id must not be empty".to_string());
    let display = format!("{error}");
    assert!(display.contains("checkout_id must not be empty"));
}

#[test]
fn repo_prep_error_display_git() {
    let error = RepoPrepError::Git {
        args: vec!["clone".to_string(), "repo".to_string()],
        status: Some(128),
        stderr: "fatal: repository not found".to_string(),
    };
    let display = format!("{error}");
    assert!(display.contains("git command failed"));
    assert!(display.contains("clone"));
    assert!(display.contains("repository not found"));
}

#[test]
fn repo_commit_request_with_author() {
    let request = RepoCommitRequest {
        message: "Fix bug".to_string(),
        author_name: Some("Alice".to_string()),
        author_email: Some("alice@example.com".to_string()),
    };
    assert_eq!(request.message, "Fix bug");
    assert_eq!(request.author_name.as_deref(), Some("Alice"));
}

#[test]
fn repo_commit_result_construction() {
    let result = RepoCommitResult {
        commit_sha: "abc123def456".to_string(),
        branch: Some("main".to_string()),
    };
    assert_eq!(result.commit_sha, "abc123def456");
}

#[test]
fn prepared_repo_construction() {
    let repo = PreparedRepo {
        checkout_root: "/tmp/checkout".into(),
        source: RepoSource::LocalPath("/tmp/source".into()),
        repository: Some("acme/project".to_string()),
        active_ref: "main".to_string(),
        branch: Some("feature".to_string()),
    };
    assert_eq!(repo.active_ref, "main");
}

#[test]
fn repo_status_defaults() {
    let status = RepoStatus {
        active_ref: "main".to_string(),
        branch: Some("main".to_string()),
        dirty: false,
        staged: false,
        untracked: false,
    };
    assert!(!status.dirty);
    assert_eq!(status.active_ref, "main");
}

#[test]
fn repo_status_dirty_flags() {
    let status = RepoStatus {
        active_ref: "feature".to_string(),
        branch: Some("feature".to_string()),
        dirty: true,
        staged: true,
        untracked: true,
    };
    assert!(status.dirty);
    assert!(status.staged);
    assert!(status.untracked);
}

#[test]
fn repo_commit_request_empty_message_is_valid() {
    let request = RepoCommitRequest {
        message: String::new(),
        author_name: None,
        author_email: None,
    };
    assert!(request.message.is_empty());
}
