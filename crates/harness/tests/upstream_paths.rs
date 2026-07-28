use std::fs;
use std::path::PathBuf;

use orbit_harness::UpstreamPaths;

#[test]
fn from_repo_root_sets_paths() {
    let root = "/tmp/orbit-test/repo";
    let paths = UpstreamPaths::from_repo_root(root);
    assert_eq!(
        paths.commands_path(),
        PathBuf::from(root).join("src/commands.ts")
    );
    assert_eq!(paths.tools_path(), PathBuf::from(root).join("src/tools.ts"));
    assert_eq!(
        paths.cli_path(),
        PathBuf::from(root).join("src/entrypoints/cli.tsx")
    );
}

#[test]
fn from_repo_root_with_trailing_slash() {
    let paths = UpstreamPaths::from_repo_root("/tmp/repo/");
    assert_eq!(
        paths.commands_path(),
        PathBuf::from("/tmp/repo/").join("src/commands.ts")
    );
}

#[test]
fn from_workspace_dir_resolves_relative_paths() {
    let tmp = temp_dir();
    let workspace = tmp.join("workspace/code");
    fs::create_dir_all(&workspace).unwrap();
    let paths = UpstreamPaths::from_workspace_dir(&workspace);
    assert!(paths
        .commands_path()
        .to_string_lossy()
        .contains("src/commands.ts"));
}

#[test]
fn from_workspace_dir_non_existent_path() {
    let tmp = temp_dir();
    let workspace = tmp.join("does-not-exist/src");
    let paths = UpstreamPaths::from_workspace_dir(&workspace);
    assert!(!paths.commands_path().as_os_str().is_empty());
}

#[test]
fn clone_and_debug() {
    let paths = UpstreamPaths::from_repo_root("/tmp/test");
    let cloned = paths.clone();
    assert_eq!(paths, cloned);
    let debug = format!("{paths:?}");
    assert!(!debug.is_empty());
}

#[test]
fn partial_eq_different_roots() {
    let a = UpstreamPaths::from_repo_root("/repo/a");
    let b = UpstreamPaths::from_repo_root("/repo/b");
    assert_ne!(a, b);
}

#[test]
fn paths_from_workspace_dir_uses_parent_for_repo_root() {
    let tmp = temp_dir();
    let inner = tmp.join("inner");
    fs::create_dir_all(&inner).unwrap();
    let paths = UpstreamPaths::from_workspace_dir(&inner);
    assert!(paths
        .commands_path()
        .to_string_lossy()
        .contains("src/commands.ts"));
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "orbit-upstream-paths-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
