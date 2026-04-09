use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoSource {
    RemoteUrl(String),
    LocalPath(PathBuf),
}

impl RepoSource {
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::RemoteUrl(url) => url.clone(),
            Self::LocalPath(path) => path.display().to_string(),
        }
    }

    fn as_arg(&self) -> &OsStr {
        match self {
            Self::RemoteUrl(url) => OsStr::new(url),
            Self::LocalPath(path) => path.as_os_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoCheckoutRequest {
    pub workspace_root: PathBuf,
    pub checkout_id: String,
    pub source: RepoSource,
    pub repository: Option<String>,
    pub base_ref: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRepo {
    pub checkout_root: PathBuf,
    pub source: RepoSource,
    pub repository: Option<String>,
    pub active_ref: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    pub active_ref: String,
    pub branch: Option<String>,
    pub dirty: bool,
    pub staged: bool,
    pub untracked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoCommitRequest {
    pub message: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoCommitResult {
    pub commit_sha: String,
    pub branch: Option<String>,
}

#[derive(Debug)]
pub enum RepoPrepError {
    Io(std::io::Error),
    InvalidRequest(String),
    Git {
        args: Vec<String>,
        status: Option<i32>,
        stderr: String,
    },
}

impl fmt::Display for RepoPrepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::InvalidRequest(message) => write!(f, "{message}"),
            Self::Git {
                args,
                status,
                stderr,
            } => {
                let status = status.map_or_else(|| "signal".to_string(), |code| code.to_string());
                write!(
                    f,
                    "git command failed (status {status}): git {}{}",
                    args.join(" "),
                    if stderr.trim().is_empty() {
                        String::new()
                    } else {
                        format!(": {}", stderr.trim())
                    }
                )
            }
        }
    }
}

impl Error for RepoPrepError {}

impl From<std::io::Error> for RepoPrepError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[must_use]
pub fn normalize_branch_name(branch: &str) -> String {
    let normalized = branch
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            '/' | '.' | '_' | '-' => '-',
            _ => '-',
        })
        .collect::<String>();
    let collapsed = normalized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if collapsed.is_empty() {
        "orbit-task".to_string()
    } else {
        collapsed
    }
}

pub fn prepare_checkout(request: &RepoCheckoutRequest) -> Result<PreparedRepo, RepoPrepError> {
    if request.checkout_id.trim().is_empty() {
        return Err(RepoPrepError::InvalidRequest(
            "checkout_id must not be empty".to_string(),
        ));
    }

    fs::create_dir_all(&request.workspace_root)?;
    let checkout_root = request
        .workspace_root
        .join(sanitize_checkout_id(&request.checkout_id));

    if checkout_root.exists() {
        ensure_checkout_root(&checkout_root)?;
        git_in(&checkout_root, ["fetch", "--all", "--prune"])?;
    } else {
        clone_into(&request.source, &checkout_root)?;
    }

    checkout_target(
        &checkout_root,
        request.base_ref.as_deref(),
        request.branch.as_deref(),
    )?;

    let active_ref = current_ref(&checkout_root)?;

    Ok(PreparedRepo {
        checkout_root,
        source: request.source.clone(),
        repository: request.repository.clone(),
        active_ref,
        branch: request.branch.clone(),
    })
}

pub fn repo_status(checkout_root: &Path) -> Result<RepoStatus, RepoPrepError> {
    ensure_checkout_root(checkout_root)?;
    let branch = git_stdout(
        checkout_root,
        ["symbolic-ref", "--quiet", "--short", "HEAD"],
    )?;
    let active_ref = branch
        .clone()
        .or_else(|| {
            git_stdout(checkout_root, ["rev-parse", "--short", "HEAD"])
                .ok()
                .flatten()
        })
        .ok_or_else(|| {
            RepoPrepError::InvalidRequest(format!(
                "could not resolve current ref for checkout {}",
                checkout_root.display()
            ))
        })?;
    let porcelain = git_stdout(checkout_root, ["status", "--porcelain"])?.unwrap_or_default();
    let mut dirty = false;
    let mut staged = false;
    let mut untracked = false;
    for line in porcelain.lines() {
        if line.is_empty() {
            continue;
        }
        dirty = true;
        if line.starts_with("??") {
            untracked = true;
            continue;
        }
        let mut chars = line.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        if index != ' ' {
            staged = true;
        }
        if worktree != ' ' {
            dirty = true;
        }
    }

    Ok(RepoStatus {
        active_ref,
        branch,
        dirty,
        staged,
        untracked,
    })
}

pub fn stage_and_commit(
    checkout_root: &Path,
    request: &RepoCommitRequest,
) -> Result<RepoCommitResult, RepoPrepError> {
    ensure_checkout_root(checkout_root)?;
    if request.message.trim().is_empty() {
        return Err(RepoPrepError::InvalidRequest(
            "commit message must not be empty".to_string(),
        ));
    }

    git_in(checkout_root, ["add", "-A"])?;
    let status = repo_status(checkout_root)?;
    if !status.dirty {
        return Err(RepoPrepError::InvalidRequest(
            "checkout has no changes to commit".to_string(),
        ));
    }

    git_in_with_env(
        checkout_root,
        ["commit", "-m", request.message.trim()],
        commit_env(request),
    )?;

    let commit_sha = git_stdout(checkout_root, ["rev-parse", "HEAD"])?.ok_or_else(|| {
        RepoPrepError::InvalidRequest(format!(
            "could not resolve commit sha after commit in {}",
            checkout_root.display()
        ))
    })?;

    Ok(RepoCommitResult {
        commit_sha,
        branch: git_stdout(
            checkout_root,
            ["symbolic-ref", "--quiet", "--short", "HEAD"],
        )?,
    })
}

pub fn push_branch(checkout_root: &Path, remote: &str, branch: &str) -> Result<(), RepoPrepError> {
    ensure_checkout_root(checkout_root)?;
    if remote.trim().is_empty() {
        return Err(RepoPrepError::InvalidRequest(
            "remote must not be empty".to_string(),
        ));
    }
    if branch.trim().is_empty() {
        return Err(RepoPrepError::InvalidRequest(
            "branch must not be empty".to_string(),
        ));
    }

    git_in(checkout_root, ["push", "--set-upstream", remote, branch])
}

fn ensure_checkout_root(checkout_root: &Path) -> Result<(), RepoPrepError> {
    if checkout_root.join(".git").exists() {
        return Ok(());
    }

    Err(RepoPrepError::InvalidRequest(format!(
        "checkout root is not a git repository: {}",
        checkout_root.display()
    )))
}

fn clone_into(source: &RepoSource, checkout_root: &Path) -> Result<(), RepoPrepError> {
    let parent = checkout_root.parent().ok_or_else(|| {
        RepoPrepError::InvalidRequest(format!(
            "checkout root has no parent: {}",
            checkout_root.display()
        ))
    })?;
    fs::create_dir_all(parent)?;

    let output = Command::new("git")
        .arg("clone")
        .arg("--")
        .arg(source.as_arg())
        .arg(checkout_root.as_os_str())
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(RepoPrepError::Git {
        args: vec![
            "clone".to_string(),
            "--".to_string(),
            source.display(),
            checkout_root.display().to_string(),
        ],
        status: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn checkout_target(
    checkout_root: &Path,
    base_ref: Option<&str>,
    branch: Option<&str>,
) -> Result<(), RepoPrepError> {
    match (base_ref, branch) {
        (Some(base_ref), Some(branch)) => {
            let resolved = resolve_base_ref(checkout_root, base_ref)?;
            git_in(checkout_root, ["checkout", "-B", branch, resolved.as_str()])?;
        }
        (Some(base_ref), None) => {
            let resolved = resolve_base_ref(checkout_root, base_ref)?;
            git_in(checkout_root, ["checkout", "--detach", resolved.as_str()])?;
        }
        (None, Some(branch)) => {
            if ref_exists(checkout_root, branch)? {
                git_in(checkout_root, ["checkout", branch])?;
            } else {
                git_in(checkout_root, ["checkout", "-b", branch])?;
            }
        }
        (None, None) => {}
    }

    Ok(())
}

fn resolve_base_ref(checkout_root: &Path, base_ref: &str) -> Result<String, RepoPrepError> {
    let remote_candidate = format!("origin/{base_ref}");
    if !base_ref.contains('/') && ref_exists(checkout_root, &remote_candidate)? {
        return Ok(remote_candidate);
    }
    if ref_exists(checkout_root, base_ref)? {
        return Ok(base_ref.to_string());
    }

    Err(RepoPrepError::InvalidRequest(format!(
        "base ref does not exist in checkout: {base_ref}"
    )))
}

fn ref_exists(checkout_root: &Path, reference: &str) -> Result<bool, RepoPrepError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout_root)
        .args(["rev-parse", "--verify", "--quiet", reference])
        .output()?;
    Ok(output.status.success())
}

fn current_ref(checkout_root: &Path) -> Result<String, RepoPrepError> {
    if let Some(branch) = git_stdout(
        checkout_root,
        ["symbolic-ref", "--quiet", "--short", "HEAD"],
    )? {
        return Ok(branch);
    }

    git_stdout(checkout_root, ["rev-parse", "--short", "HEAD"])?.ok_or_else(|| {
        RepoPrepError::InvalidRequest(format!(
            "could not resolve current ref for checkout {}",
            checkout_root.display()
        ))
    })
}

fn commit_env(request: &RepoCommitRequest) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if let Some(name) = request
        .author_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        env.push(("GIT_AUTHOR_NAME".to_string(), name.trim().to_string()));
        env.push(("GIT_COMMITTER_NAME".to_string(), name.trim().to_string()));
    }
    if let Some(email) = request
        .author_email
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        env.push(("GIT_AUTHOR_EMAIL".to_string(), email.trim().to_string()));
        env.push(("GIT_COMMITTER_EMAIL".to_string(), email.trim().to_string()));
    }
    env
}

fn git_stdout<const N: usize>(
    checkout_root: &Path,
    args: [&str; N],
) -> Result<Option<String>, RepoPrepError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout_root)
        .args(args)
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!stdout.is_empty()).then_some(stdout))
}

fn git_in<const N: usize>(checkout_root: &Path, args: [&str; N]) -> Result<(), RepoPrepError> {
    git_in_with_env(checkout_root, args, Vec::new())
}

fn git_in_with_env<const N: usize>(
    checkout_root: &Path,
    args: [&str; N],
    env_vars: Vec<(String, String)>,
) -> Result<(), RepoPrepError> {
    let mut command = Command::new("git");
    command.arg("-C").arg(checkout_root).args(args);
    for (key, value) in env_vars {
        command.env(key, value);
    }
    let output = command.output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(RepoPrepError::Git {
        args: std::iter::once("-C".to_string())
            .chain(std::iter::once(checkout_root.display().to_string()))
            .chain(args.into_iter().map(str::to_string))
            .collect(),
        status: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn sanitize_checkout_id(checkout_id: &str) -> String {
    let sanitized = checkout_id
        .trim()
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => character,
            _ => '_',
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "checkout".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn prepare_checkout_clones_local_repo_and_creates_branch_from_base_ref() {
        let root = temp_dir("orbit-repo-clone");
        let source = init_git_repo(root.join("source"));
        commit_file(&source, "README.md", "hello from main\n", "initial commit");

        let prepared = prepare_checkout(&RepoCheckoutRequest {
            workspace_root: root.join("workspaces"),
            checkout_id: "task-123".to_string(),
            source: RepoSource::LocalPath(source.clone()),
            repository: Some("acme/payments".to_string()),
            base_ref: Some("main".to_string()),
            branch: Some("orbit/task-123".to_string()),
        })
        .expect("checkout should prepare");

        assert_eq!(prepared.branch.as_deref(), Some("orbit/task-123"));
        assert_eq!(prepared.active_ref, "orbit/task-123");
        assert_eq!(
            fs::read_to_string(prepared.checkout_root.join("README.md")).unwrap(),
            "hello from main\n"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_checkout_fetches_latest_remote_state_when_reused() {
        let root = temp_dir("orbit-repo-fetch");
        let source = init_git_repo(root.join("source"));
        commit_file(&source, "README.md", "v1\n", "initial commit");

        let request = RepoCheckoutRequest {
            workspace_root: root.join("workspaces"),
            checkout_id: "task-fetch".to_string(),
            source: RepoSource::LocalPath(source.clone()),
            repository: Some("acme/payments".to_string()),
            base_ref: Some("main".to_string()),
            branch: Some("orbit/task-fetch".to_string()),
        };

        let first = prepare_checkout(&request).expect("first checkout should prepare");
        assert_eq!(
            fs::read_to_string(first.checkout_root.join("README.md")).unwrap(),
            "v1\n"
        );

        commit_file(&source, "README.md", "v2\n", "update main");

        let second = prepare_checkout(&request).expect("second checkout should refetch");
        assert_eq!(
            fs::read_to_string(second.checkout_root.join("README.md")).unwrap(),
            "v2\n"
        );
        assert_eq!(second.active_ref, "orbit/task-fetch");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_checkout_supports_detached_base_ref_without_branch() {
        let root = temp_dir("orbit-repo-detached");
        let source = init_git_repo(root.join("source"));
        commit_file(&source, "README.md", "detached\n", "initial commit");

        let prepared = prepare_checkout(&RepoCheckoutRequest {
            workspace_root: root.join("workspaces"),
            checkout_id: "task-detached".to_string(),
            source: RepoSource::LocalPath(source.clone()),
            repository: None,
            base_ref: Some("main".to_string()),
            branch: None,
        })
        .expect("detached checkout should prepare");

        assert_ne!(prepared.active_ref, "main");
        assert_eq!(
            fs::read_to_string(prepared.checkout_root.join("README.md")).unwrap(),
            "detached\n"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn normalize_branch_name_sanitizes_and_collapses_segments() {
        assert_eq!(
            normalize_branch_name(" Feature 123 / Fix::Bug "),
            "feature-123-fix-bug"
        );
        assert_eq!(normalize_branch_name("////"), "orbit-task");
    }

    #[test]
    fn repo_status_reports_dirty_and_untracked_state() {
        let root = temp_dir("orbit-repo-status");
        let repo = init_git_repo(root.join("repo"));
        commit_file(&repo, "README.md", "clean\n", "initial commit");

        let clean = repo_status(&repo).expect("status should load");
        assert_eq!(clean.branch.as_deref(), Some("main"));
        assert!(!clean.dirty);
        assert!(!clean.staged);
        assert!(!clean.untracked);

        fs::write(repo.join("notes.txt"), "new file\n").unwrap();
        let dirty = repo_status(&repo).expect("dirty status should load");
        assert!(dirty.dirty);
        assert!(dirty.untracked);
        assert!(!dirty.staged);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stage_and_commit_and_push_branch_updates_remote() {
        let root = temp_dir("orbit-repo-push");
        let remote = root.join("remote.git");
        run_git_in_dir(root.as_path(), ["init", "--bare", remote.to_str().unwrap()]);
        run_git_in_dir(
            remote.as_path(),
            ["symbolic-ref", "HEAD", "refs/heads/main"],
        );

        let source = root.join("source");
        run_git_in_dir(
            root.as_path(),
            ["clone", remote.to_str().unwrap(), source.to_str().unwrap()],
        );
        run_git(&source, ["config", "user.name", "Orbit Tests"]);
        run_git(&source, ["config", "user.email", "orbit-tests@example.com"]);
        commit_file(&source, "README.md", "main\n", "initial commit");
        run_git(&source, ["push", "-u", "origin", "main"]);

        let prepared = prepare_checkout(&RepoCheckoutRequest {
            workspace_root: root.join("workspaces"),
            checkout_id: "task-push".to_string(),
            source: RepoSource::LocalPath(remote.clone()),
            repository: Some("acme/payments".to_string()),
            base_ref: Some("main".to_string()),
            branch: Some("orbit/task-push".to_string()),
        })
        .expect("checkout should prepare");

        fs::write(
            prepared.checkout_root.join("README.md"),
            "updated from worker\n",
        )
        .unwrap();
        let commit = stage_and_commit(
            &prepared.checkout_root,
            &RepoCommitRequest {
                message: "Update from hosted worker".to_string(),
                author_name: Some("Orbit Worker".to_string()),
                author_email: Some("orbit-worker@example.com".to_string()),
            },
        )
        .expect("commit should succeed");
        assert_eq!(commit.branch.as_deref(), Some("orbit/task-push"));

        push_branch(&prepared.checkout_root, "origin", "orbit/task-push")
            .expect("push should succeed");

        let verify = root.join("verify");
        run_git_in_dir(
            root.as_path(),
            ["clone", remote.to_str().unwrap(), verify.to_str().unwrap()],
        );
        run_git(&verify, ["checkout", "orbit/task-push"]);
        assert_eq!(
            fs::read_to_string(verify.join("README.md")).unwrap(),
            "updated from worker\n"
        );

        let _ = fs::remove_dir_all(root);
    }

    fn init_git_repo(path: PathBuf) -> PathBuf {
        fs::create_dir_all(&path).unwrap();
        run_git(&path, ["init", "-b", "main"]);
        run_git(&path, ["config", "user.name", "Orbit Tests"]);
        run_git(&path, ["config", "user.email", "orbit-tests@example.com"]);
        path
    }

    fn commit_file(repo: &Path, relative_path: &str, contents: &str, message: &str) {
        let file_path = repo.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, contents).unwrap();
        run_git(repo, ["add", relative_path]);
        run_git(repo, ["commit", "-m", message]);
    }

    fn run_git<const N: usize>(repo: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git command failed: git -C {} {}: {}",
            repo.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_git_in_dir<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git command failed in {}: git {}: {}",
            cwd.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{millis}-{counter}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
