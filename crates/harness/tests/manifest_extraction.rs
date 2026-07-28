use std::fs;
use std::path::PathBuf;

use orbit_harness::{extract_bootstrap_plan, extract_commands, extract_tools, UpstreamPaths};

#[test]
fn extract_commands_parses_imports() {
    let source = r#"
import { HelpCommand, StatusCommand } from "./commands/help";
import { DiffCommand } from "./commands/diff";
"#;
    let registry = extract_commands(source);
    let entries = registry.entries();
    assert_eq!(entries.len(), 3);
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"HelpCommand"));
    assert!(names.contains(&"StatusCommand"));
    assert!(names.contains(&"DiffCommand"));
    for entry in entries {
        assert_eq!(entry.source, orbit_commands::CommandSource::Builtin);
    }
}

#[test]
fn extract_commands_detects_internal_block() {
    let source = r"
export const INTERNAL_ONLY_COMMANDS = [
    InternalDebugCommand,
    SecretCommand,
]
";
    let registry = extract_commands(source);
    let entries = registry.entries();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(entry.source, orbit_commands::CommandSource::InternalOnly);
    }
}

#[test]
fn extract_commands_detects_feature_gated() {
    let source = r"BetaFeature = feature('beta') && require('./commands/beta');";
    let registry = extract_commands(source);
    assert!(!registry.entries().is_empty());
    let entry = &registry.entries()[0];
    assert_eq!(entry.name, "BetaFeature");
    assert_eq!(entry.source, orbit_commands::CommandSource::FeatureGated);
}

#[test]
fn extract_commands_empty_source() {
    let registry = extract_commands("");
    assert!(registry.entries().is_empty());
}

#[test]
fn extract_commands_no_matches() {
    let registry = extract_commands("fn main() { }");
    assert!(registry.entries().is_empty());
}

#[test]
fn extract_tools_parses_imports() {
    let source = r#"
import { BashTool, FileEditTool } from "./tools/bash";
import { ReadTool } from "./tools/read";
"#;
    let registry = extract_tools(source);
    let entries = registry.entries();
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"BashTool"));
    assert!(names.contains(&"FileEditTool"));
    assert!(names.contains(&"ReadTool"));
    for entry in entries {
        assert_eq!(entry.source, orbit_tools::ToolSource::Base);
    }
}

#[test]
fn extract_tools_ignores_non_tool_imports() {
    let source = r#"
import { Config } from "../config";
import { Helper } from "./utils/helper";
"#;
    let registry = extract_tools(source);
    // Non-tool imports from ./tools/ won't match, those from other paths are ignored
    assert!(registry.entries().is_empty());
}

#[test]
fn extract_tools_detects_conditional_tools() {
    let source = r"BetaTool = feature('beta') && require('./tools/beta');";
    let registry = extract_tools(source);
    assert!(registry
        .entries()
        .iter()
        .any(|e| e.name == "BetaTool" && e.source == orbit_tools::ToolSource::Conditional));
}

#[test]
fn extract_tools_empty_source() {
    let registry = extract_tools("");
    assert!(registry.entries().is_empty());
}

#[test]
fn extract_tools_deduplicates() {
    let source = r#"
import { BashTool } from "./tools/bash";
import { BashTool } from "./tools/bash";
"#;
    let registry = extract_tools(source);
    assert_eq!(registry.entries().len(), 1);
}

#[test]
fn extract_bootstrap_plan_minimal() {
    let source = "console.log('start');";
    let plan = extract_bootstrap_plan(source);
    let phases = plan.phases();
    assert!(!phases.is_empty());
    assert_eq!(phases[0], orbit_runtime::BootstrapPhase::CliEntry);
    assert_eq!(
        phases[phases.len() - 1],
        orbit_runtime::BootstrapPhase::MainRuntime
    );
}

#[test]
fn extract_bootstrap_plan_with_version() {
    let source = r"
if (args.includes('--version')) { process.exit(0); }
";
    let plan = extract_bootstrap_plan(source);
    let phases = plan.phases();
    assert!(phases.contains(&orbit_runtime::BootstrapPhase::FastPathVersion));
}

#[test]
fn extract_bootstrap_plan_with_startup_profiler() {
    let source = "startupProfiler.begin();";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::StartupProfiler));
}

#[test]
fn extract_bootstrap_plan_with_dump_system_prompt() {
    let source = "--dump-system-prompt";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::SystemPromptFastPath));
}

#[test]
fn extract_bootstrap_plan_with_chrome_mcp() {
    let source = "--claude-in-chrome-mcp";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::ChromeMcpFastPath));
}

#[test]
fn extract_bootstrap_plan_with_daemon_worker() {
    let source = "--daemon-worker";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::DaemonWorkerFastPath));
}

#[test]
fn extract_bootstrap_plan_with_remote_control() {
    let source = "remote-control";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::BridgeFastPath));
}

#[test]
fn extract_bootstrap_plan_with_daemon() {
    let source = "args[0] === 'daemon'";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::DaemonFastPath));
}

#[test]
fn extract_bootstrap_plan_with_background_session() {
    let source = r"args[0] === 'ps'";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::BackgroundSessionFastPath));
}

#[test]
fn extract_bootstrap_plan_with_template_fast_path() {
    let source = "args[0] === 'new' || args[0] === 'list' || args[0] === 'reply'";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::TemplateFastPath));
}

#[test]
fn extract_bootstrap_plan_with_environment_runner() {
    let source = "environment-runner";
    let plan = extract_bootstrap_plan(source);
    assert!(plan
        .phases()
        .contains(&orbit_runtime::BootstrapPhase::EnvironmentRunnerFastPath));
}

#[test]
fn upstream_paths_from_repo_root() {
    let tmp = temp_dir();
    let repo_root = tmp.join("orbit-repo");
    let paths = UpstreamPaths::from_repo_root(&repo_root);
    assert_eq!(paths.commands_path(), repo_root.join("src/commands.ts"));
    assert_eq!(paths.tools_path(), repo_root.join("src/tools.ts"));
    assert_eq!(paths.cli_path(), repo_root.join("src/entrypoints/cli.tsx"));
}

#[test]
fn upstream_paths_from_workspace_dir() {
    let tmp = temp_dir();
    let workspace = tmp.join("workspace/dir");
    fs::create_dir_all(&workspace).unwrap();
    let paths = UpstreamPaths::from_workspace_dir(&workspace);
    assert!(!paths.commands_path().as_os_str().is_empty());
}

#[test]
fn upstream_paths_equality() {
    let a = UpstreamPaths::from_repo_root("/a/b/c");
    let b = UpstreamPaths::from_repo_root("/a/b/c");
    assert_eq!(a, b);
    let c = UpstreamPaths::from_repo_root("/x/y/z");
    assert_ne!(a, c);
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "orbit-harness-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
