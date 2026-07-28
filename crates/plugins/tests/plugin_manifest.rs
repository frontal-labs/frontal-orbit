use std::fs;
use std::path::{Path, PathBuf};

use orbit_plugins::{
    load_plugin_from_directory, PluginCommandManifest, PluginError, PluginHooks, PluginLifecycle,
    PluginManifest, PluginPermission,
};

fn temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("plugins-manifest-{label}-{nanos}"))
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent dir");
    }
    fs::write(path, contents).expect("write file");
}

fn write_hook_script(root: &Path, name: &str, content: &str) {
    let path = root.join("hooks").join(name);
    write_file(&path, content);
}

fn write_tool_script(root: &Path, name: &str, content: &str) {
    let path = root.join("tools").join(name);
    write_file(&path, content);
}

#[test]
fn loads_valid_manifest_with_all_fields() {
    let root = temp_dir("valid");
    write_hook_script(&root, "pre.sh", "#!/bin/sh\necho pre");
    write_tool_script(&root, "echo.sh", "#!/bin/sh\ncat");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "full-plugin",
            "version": "2.0.0",
            "description": "A plugin with everything",
            "defaultEnabled": true,
            "permissions": ["read", "write", "execute"],
            "hooks": {
                "PreToolUse": ["./hooks/pre.sh"],
                "PostToolUse": ["./hooks/pre.sh"],
                "PostToolUseFailure": ["./hooks/pre.sh"]
            },
            "lifecycle": {
                "Init": ["./hooks/pre.sh"],
                "Shutdown": ["./hooks/pre.sh"]
            },
            "tools": [
                {
                    "name": "my_tool",
                    "description": "A test tool",
                    "inputSchema": {"type": "object"},
                    "command": "./tools/echo.sh",
                    "requiredPermission": "read-only"
                }
            ],
            "commands": [
                {
                    "name": "greet",
                    "description": "Say hello",
                    "command": "./hooks/pre.sh"
                }
            ]
        }"#,
    );
    let manifest = load_plugin_from_directory(&root).expect("should load");
    assert_eq!(manifest.name, "full-plugin");
    assert_eq!(manifest.version, "2.0.0");
    assert!(manifest.default_enabled);
    assert_eq!(manifest.permissions.len(), 3);
    assert!(!manifest.hooks.is_empty());
    assert!(!manifest.lifecycle.is_empty());
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.commands.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn loads_minimal_manifest() {
    let root = temp_dir("minimal");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "minimal",
            "version": "0.1.0",
            "description": "Tiny plugin"
        }"#,
    );
    let manifest = load_plugin_from_directory(&root).expect("should load");
    assert_eq!(manifest.name, "minimal");
    assert!(manifest.permissions.is_empty());
    assert!(manifest.hooks.is_empty());
    assert!(manifest.lifecycle.is_empty());
    assert!(manifest.tools.is_empty());
    assert!(manifest.commands.is_empty());
    assert!(!manifest.default_enabled);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_empty_name() {
    let root = temp_dir("empty-name");
    write_file(
        &root.join("plugin.json"),
        r#"{"name":"","version":"1.0.0","description":"desc"}"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(matches!(error, PluginError::ManifestValidation(_)));
    assert!(error.to_string().contains("name cannot be empty"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_empty_version() {
    let root = temp_dir("empty-version");
    write_file(
        &root.join("plugin.json"),
        r#"{"name":"test","version":"","description":"desc"}"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("version cannot be empty"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_empty_description() {
    let root = temp_dir("empty-desc");
    write_file(
        &root.join("plugin.json"),
        r#"{"name":"test","version":"1.0.0","description":""}"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("description cannot be empty"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_invalid_permissions() {
    let root = temp_dir("bad-perms");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "bad-perms",
            "version": "1.0.0",
            "description": "desc",
            "permissions": ["admin"]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error
        .to_string()
        .contains("must be one of read, write, or execute"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_duplicate_permissions() {
    let root = temp_dir("dup-perms");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "dup-perms",
            "version": "1.0.0",
            "description": "desc",
            "permissions": ["read", "read"]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("is duplicated"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_duplicate_tools() {
    let root = temp_dir("dup-tools");
    write_tool_script(&root, "tool.sh", "#!/bin/sh\ncat");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "dup-tools",
            "version": "1.0.0",
            "description": "desc",
            "tools": [
                {"name": "same", "description": "first", "inputSchema": {"type":"object"}, "command": "./tools/tool.sh"},
                {"name": "same", "description": "second", "inputSchema": {"type":"object"}, "command": "./tools/tool.sh"}
            ]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("is duplicated"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_duplicate_commands() {
    let root = temp_dir("dup-cmds");
    write_hook_script(&root, "cmd.sh", "#!/bin/sh\necho cmd");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "dup-cmds",
            "version": "1.0.0",
            "description": "desc",
            "commands": [
                {"name": "dup", "description": "first", "command": "./hooks/cmd.sh"},
                {"name": "dup", "description": "second", "command": "./hooks/cmd.sh"}
            ]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("is duplicated"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_missing_hook_path() {
    let root = temp_dir("missing-hook");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "missing-hook",
            "version": "1.0.0",
            "description": "desc",
            "hooks": {"PreToolUse": ["./hooks/missing.sh"]}
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("does not exist"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_missing_tool_path() {
    let root = temp_dir("missing-tool");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "missing-tool",
            "version": "1.0.0",
            "description": "desc",
            "tools": [
                {
                    "name": "broken",
                    "description": "broken tool",
                    "inputSchema": {"type": "object"},
                    "command": "./tools/missing.sh"
                }
            ]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("does not exist"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_directory_as_hook_path() {
    let root = temp_dir("dir-hook");
    fs::create_dir_all(root.join("hooks").join("pre-dir")).expect("create dir");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "dir-hook",
            "version": "1.0.0",
            "description": "desc",
            "hooks": {"PreToolUse": ["./hooks/pre-dir"]}
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error.to_string().contains("must point to a file"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_invalid_tool_required_permission() {
    let root = temp_dir("bad-tool-perm");
    write_tool_script(&root, "tool.sh", "#!/bin/sh\ncat");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "bad-tool-perm",
            "version": "1.0.0",
            "description": "desc",
            "tools": [
                {
                    "name": "bad_perm",
                    "description": "bad",
                    "inputSchema": {"type": "object"},
                    "command": "./tools/tool.sh",
                    "requiredPermission": "super-admin"
                }
            ]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error
        .to_string()
        .contains("must be read-only, workspace-write, or danger-full-access"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_invalid_tool_input_schema() {
    let root = temp_dir("bad-schema");
    write_tool_script(&root, "tool.sh", "#!/bin/sh\ncat");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "bad-schema",
            "version": "1.0.0",
            "description": "desc",
            "tools": [
                {
                    "name": "bad_schema",
                    "description": "bad schema",
                    "inputSchema": "not-an-object",
                    "command": "./tools/tool.sh"
                }
            ]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error
        .to_string()
        .contains("inputSchema must be a JSON object"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_claude_code_contract_fields() {
    let root = temp_dir("claude-contract");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "cc-plugin",
            "version": "1.0.0",
            "description": "Claude Code contract test",
            "skills": "./skills/",
            "mcpServers": "./.mcp.json",
            "agents": ["agents/*.md"]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    let msg = error.to_string();
    assert!(msg.contains("field `skills` uses the Claude Code plugin contract"));
    assert!(msg.contains("field `mcpServers` uses the Claude Code plugin contract"));
    assert!(msg.contains("field `agents` uses the Claude Code plugin contract"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_unsupported_hook_names() {
    let root = temp_dir("bad-hooks");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "bad-hooks",
            "version": "1.0.0",
            "description": "unsupported hooks",
            "hooks": {
                "SessionStart": ["scripts/start.mjs"]
            }
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    assert!(error
        .to_string()
        .contains("hook `SessionStart` uses the Claude Code lifecycle contract"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn loads_packaged_manifest_path() {
    let root = temp_dir("packaged");
    write_hook_script(&root, "pre.sh", "#!/bin/sh\necho pre");
    fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
    write_file(
        &root.join(".claude-plugin").join("plugin.json"),
        r#"{
            "name": "packaged-plugin",
            "version": "1.0.0",
            "description": "Packaged manifest test",
            "hooks": {"PreToolUse": ["./hooks/pre.sh"]}
        }"#,
    );
    let manifest = load_plugin_from_directory(&root).expect("should load packaged manifest");
    assert_eq!(manifest.name, "packaged-plugin");
    assert_eq!(manifest.hooks.pre_tool_use.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn manifest_serde_roundtrip() {
    let manifest = PluginManifest {
        name: "roundtrip".to_string(),
        version: "1.0.0".to_string(),
        description: "Roundtrip test".to_string(),
        permissions: vec![PluginPermission::Read, PluginPermission::Write],
        default_enabled: true,
        hooks: PluginHooks {
            pre_tool_use: vec!["echo pre".to_string()],
            post_tool_use: vec![],
            post_tool_use_failure: vec![],
        },
        lifecycle: PluginLifecycle::default(),
        tools: vec![],
        commands: vec![PluginCommandManifest {
            name: "test".to_string(),
            description: "test cmd".to_string(),
            command: "echo test".to_string(),
        }],
    };
    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["name"], "roundtrip");
    assert_eq!(json["version"], "1.0.0");
    assert_eq!(json["permissions"][0], "read");
    assert_eq!(json["permissions"][1], "write");
    assert_eq!(json["hooks"]["PreToolUse"][0], "echo pre");
    assert_eq!(json["commands"][0]["name"], "test");

    let deserialized: PluginManifest = serde_json::from_value(json).unwrap();
    assert_eq!(manifest, deserialized);
}

#[test]
fn accumulates_multiple_validation_errors() {
    let root = temp_dir("multi-error");
    write_file(
        &root.join("plugin.json"),
        r#"{
            "name": "",
            "version": "",
            "description": "",
            "permissions": ["admin"]
        }"#,
    );
    let error = load_plugin_from_directory(&root).expect_err("should fail");
    match &error {
        PluginError::ManifestValidation(errors) => {
            assert!(
                errors.len() >= 4,
                "expected at least 4 errors, got {}",
                errors.len()
            );
        }
        other => panic!("expected ManifestValidation, got {other}"),
    }

    let _ = fs::remove_dir_all(root);
}
