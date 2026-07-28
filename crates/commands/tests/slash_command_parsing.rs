use orbit_commands::{validate_slash_command_input, SlashCommand, SlashCommandParseError};

fn parse(input: &str) -> Result<Option<SlashCommand>, SlashCommandParseError> {
    validate_slash_command_input(input)
}

#[test]
fn non_slash_input_returns_ok_none() {
    assert_eq!(parse("hello").unwrap(), None);
    assert_eq!(parse("").unwrap(), None);
    assert_eq!(parse("   ").unwrap(), None);
    assert_eq!(parse("a/b").unwrap(), None);
}

#[test]
fn empty_slash_returns_error() {
    let err = parse("/").unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn slash_help() {
    assert_eq!(parse("/help").unwrap(), Some(SlashCommand::Help));
}

#[test]
fn slash_help_with_args_is_error() {
    assert!(parse("/help extra").is_err());
}

#[test]
fn slash_status() {
    assert_eq!(parse("/status").unwrap(), Some(SlashCommand::Status));
}

#[test]
fn slash_compact() {
    assert_eq!(parse("/compact").unwrap(), Some(SlashCommand::Compact));
}

#[test]
fn slash_config() {
    assert_eq!(
        parse("/config").unwrap(),
        Some(SlashCommand::Config { section: None })
    );
    assert_eq!(
        parse("/config env").unwrap(),
        Some(SlashCommand::Config {
            section: Some("env".to_string())
        })
    );
}

#[test]
fn slash_model() {
    assert_eq!(
        parse("/model").unwrap(),
        Some(SlashCommand::Model { model: None })
    );
    assert_eq!(
        parse("/model claude-4").unwrap(),
        Some(SlashCommand::Model {
            model: Some("claude-4".to_string())
        })
    );
}

#[test]
fn slash_model_too_many_args_is_error() {
    assert!(parse("/model a b").is_err());
}

#[test]
fn slash_clear() {
    assert_eq!(
        parse("/clear").unwrap(),
        Some(SlashCommand::Clear { confirm: false })
    );
    assert_eq!(
        parse("/clear --confirm").unwrap(),
        Some(SlashCommand::Clear { confirm: true })
    );
}

#[test]
fn slash_clear_bad_arg_is_error() {
    assert!(parse("/clear --force").is_err());
    assert!(parse("/clear a b").is_err());
}

#[test]
fn slash_session_list() {
    assert_eq!(
        parse("/session").unwrap(),
        Some(SlashCommand::Session {
            action: None,
            target: None
        })
    );
    assert_eq!(
        parse("/session list").unwrap(),
        Some(SlashCommand::Session {
            action: Some("list".to_string()),
            target: None
        })
    );
}

#[test]
fn slash_session_switch() {
    assert_eq!(
        parse("/session switch abc").unwrap(),
        Some(SlashCommand::Session {
            action: Some("switch".to_string()),
            target: Some("abc".to_string())
        })
    );
}

#[test]
fn slash_session_switch_missing_arg_is_error() {
    assert!(parse("/session switch").is_err());
}

#[test]
fn slash_session_fork() {
    assert_eq!(
        parse("/session fork").unwrap(),
        Some(SlashCommand::Session {
            action: Some("fork".to_string()),
            target: None
        })
    );
    assert_eq!(
        parse("/session fork my-branch").unwrap(),
        Some(SlashCommand::Session {
            action: Some("fork".to_string()),
            target: Some("my-branch".to_string())
        })
    );
}

#[test]
fn slash_session_unknown_action_is_error() {
    assert!(parse("/session unknown").is_err());
}

#[test]
fn slash_mcp_list() {
    assert_eq!(
        parse("/mcp").unwrap(),
        Some(SlashCommand::Mcp {
            action: None,
            target: None
        })
    );
    assert_eq!(
        parse("/mcp list").unwrap(),
        Some(SlashCommand::Mcp {
            action: Some("list".to_string()),
            target: None
        })
    );
}

#[test]
fn slash_mcp_show() {
    assert_eq!(
        parse("/mcp show my-server").unwrap(),
        Some(SlashCommand::Mcp {
            action: Some("show".to_string()),
            target: Some("my-server".to_string())
        })
    );
}

#[test]
fn slash_mcp_show_missing_arg_is_error() {
    assert!(parse("/mcp show").is_err());
}

#[test]
fn slash_mcp_help() {
    assert_eq!(
        parse("/mcp help").unwrap(),
        Some(SlashCommand::Mcp {
            action: Some("help".to_string()),
            target: None
        })
    );
}

#[test]
fn slash_mcp_unknown_action_is_error() {
    assert!(parse("/mcp unknown").is_err());
}

#[test]
fn slash_permissions() {
    assert_eq!(
        parse("/permissions").unwrap(),
        Some(SlashCommand::Permissions { mode: None })
    );
    assert_eq!(
        parse("/permissions read-only").unwrap(),
        Some(SlashCommand::Permissions {
            mode: Some("read-only".to_string())
        })
    );
    assert_eq!(
        parse("/permissions workspace-write").unwrap(),
        Some(SlashCommand::Permissions {
            mode: Some("workspace-write".to_string())
        })
    );
    assert_eq!(
        parse("/permissions danger-full-access").unwrap(),
        Some(SlashCommand::Permissions {
            mode: Some("danger-full-access".to_string())
        })
    );
}

#[test]
fn slash_permissions_invalid_mode_is_error() {
    assert!(parse("/permissions whatever").is_err());
}

#[test]
fn slash_telemetry() {
    assert_eq!(
        parse("/telemetry").unwrap(),
        Some(SlashCommand::Telemetry {
            action: None,
            target: None
        })
    );
    assert_eq!(
        parse("/telemetry status").unwrap(),
        Some(SlashCommand::Telemetry {
            action: Some("status".to_string()),
            target: None
        })
    );
    assert_eq!(
        parse("/telemetry status project").unwrap(),
        Some(SlashCommand::Telemetry {
            action: Some("status".to_string()),
            target: Some("project".to_string())
        })
    );
    assert_eq!(
        parse("/telemetry on").unwrap(),
        Some(SlashCommand::Telemetry {
            action: Some("on".to_string()),
            target: None
        })
    );
    assert_eq!(
        parse("/telemetry off local").unwrap(),
        Some(SlashCommand::Telemetry {
            action: Some("off".to_string()),
            target: Some("local".to_string())
        })
    );
}

#[test]
fn slash_telemetry_unknown_action_is_error() {
    assert!(parse("/telemetry unknown").is_err());
}

#[test]
fn slash_plugin_install() {
    assert_eq!(
        parse("/plugin").unwrap(),
        Some(SlashCommand::Plugins {
            action: None,
            target: None
        })
    );
    assert_eq!(
        parse("/plugin list").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("list".to_string()),
            target: None
        })
    );
    assert_eq!(
        parse("/plugin install /path/to/plugin").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("install".to_string()),
            target: Some("/path/to/plugin".to_string())
        })
    );
    assert_eq!(
        parse("/plugins install /path").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("install".to_string()),
            target: Some("/path".to_string())
        })
    );
}

#[test]
fn slash_plugin_enable_disable() {
    assert_eq!(
        parse("/plugin enable my-plugin").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("enable".to_string()),
            target: Some("my-plugin".to_string())
        })
    );
    assert_eq!(
        parse("/plugin disable my-plugin").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("disable".to_string()),
            target: Some("my-plugin".to_string())
        })
    );
    assert!(parse("/plugin enable").is_err());
}

#[test]
fn slash_plugin_uninstall_update() {
    assert_eq!(
        parse("/plugin uninstall plugin-id").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("uninstall".to_string()),
            target: Some("plugin-id".to_string())
        })
    );
    assert_eq!(
        parse("/plugin update plugin-id").unwrap(),
        Some(SlashCommand::Plugins {
            action: Some("update".to_string()),
            target: Some("plugin-id".to_string())
        })
    );
    assert!(parse("/plugin uninstall").is_err());
}

#[test]
fn slash_plugin_unknown_action_is_error() {
    assert!(parse("/plugin unknown").is_err());
}

#[test]
fn slash_ide() {
    assert_eq!(
        parse("/ide").unwrap(),
        Some(SlashCommand::Ide { target: None })
    );
    assert_eq!(
        parse("/ide vscode").unwrap(),
        Some(SlashCommand::Ide {
            target: Some("vscode".to_string())
        })
    );
    assert_eq!(
        parse("/ide cursor").unwrap(),
        Some(SlashCommand::Ide {
            target: Some("cursor".to_string())
        })
    );
}

#[test]
fn slash_ide_invalid_target_is_error() {
    assert!(parse("/ide invalid").is_err());
}

#[test]
fn slash_args_subcommands() {
    assert_eq!(
        parse("/agents").unwrap(),
        Some(SlashCommand::Agents { args: None })
    );
    assert_eq!(
        parse("/agents list").unwrap(),
        Some(SlashCommand::Agents {
            args: Some("list".to_string())
        })
    );
    assert_eq!(
        parse("/skills").unwrap(),
        Some(SlashCommand::Skills { args: None })
    );
    assert_eq!(
        parse("/skills list").unwrap(),
        Some(SlashCommand::Skills {
            args: Some("list".to_string())
        })
    );
}

#[test]
fn slash_no_args_commands() {
    assert_eq!(parse("/diff").unwrap(), Some(SlashCommand::Diff));
    assert_eq!(parse("/version").unwrap(), Some(SlashCommand::Version));
    assert_eq!(parse("/memory").unwrap(), Some(SlashCommand::Memory));
    assert_eq!(parse("/init").unwrap(), Some(SlashCommand::Init));
    assert_eq!(parse("/doctor").unwrap(), Some(SlashCommand::Doctor));
    assert_eq!(parse("/vim").unwrap(), Some(SlashCommand::Vim));
    assert_eq!(parse("/cost").unwrap(), Some(SlashCommand::Cost));
    assert_eq!(parse("/exit").unwrap(), Some(SlashCommand::Exit));
    assert_eq!(parse("/stats").unwrap(), Some(SlashCommand::Stats));
    assert_eq!(parse("/files").unwrap(), Some(SlashCommand::Files));
    assert_eq!(parse("/fast").unwrap(), Some(SlashCommand::Fast));
    assert_eq!(parse("/share").unwrap(), Some(SlashCommand::Share));
    assert_eq!(parse("/feedback").unwrap(), Some(SlashCommand::Feedback));
    assert_eq!(parse("/summary").unwrap(), Some(SlashCommand::Summary));
    assert_eq!(parse("/brief").unwrap(), Some(SlashCommand::Brief));
    assert_eq!(parse("/advisor").unwrap(), Some(SlashCommand::Advisor));
}

#[test]
fn slash_no_args_commands_with_args_is_error() {
    assert!(parse("/diff x").is_err());
    assert!(parse("/cost x").is_err());
    assert!(parse("/memory x").is_err());
}

#[test]
fn slash_unknown_command() {
    assert_eq!(
        parse("/somegarbage").unwrap(),
        Some(SlashCommand::Unknown("somegarbage".to_string()))
    );
}

#[test]
fn slash_command_parse_method() {
    assert_eq!(
        SlashCommand::parse("/help").unwrap(),
        Some(SlashCommand::Help)
    );
    assert!(SlashCommand::parse("/").is_err());
}
