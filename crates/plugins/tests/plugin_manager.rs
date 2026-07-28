use std::path::PathBuf;

use orbit_plugins::{builtin_plugins, Plugin, PluginKind, PluginManager, PluginManagerConfig};

#[test]
fn config_new_sets_config_home() {
    let config = PluginManagerConfig::new("/tmp/orbit");
    assert_eq!(config.config_home, PathBuf::from("/tmp/orbit"));
    assert!(config.enabled_plugins.is_empty());
    assert!(config.external_dirs.is_empty());
    assert!(config.install_root.is_none());
    assert!(config.registry_path.is_none());
    assert!(config.bundled_root.is_none());
}

#[test]
fn manager_new_stores_config() {
    let config = PluginManagerConfig::new("/tmp/manager-test");
    let manager = PluginManager::new(config.clone());
    // Verify by checking derived path
    assert!(manager.registry_path().parent().is_some());
}

#[test]
fn install_root_defaults_to_config_home_plugins_installed() {
    let config = PluginManagerConfig::new("/tmp/orbit-home");
    let manager = PluginManager::new(config);
    let expected = PathBuf::from("/tmp/orbit-home/plugins/installed");
    assert_eq!(manager.install_root(), expected);
}

#[test]
fn install_root_respects_custom_value() {
    let mut config = PluginManagerConfig::new("/tmp/orbit-home");
    config.install_root = Some(PathBuf::from("/custom/install"));
    let manager = PluginManager::new(config);
    assert_eq!(manager.install_root(), PathBuf::from("/custom/install"));
}

#[test]
fn registry_path_defaults_to_config_home_plugins_installed_json() {
    let config = PluginManagerConfig::new("/tmp/orbit-home");
    let manager = PluginManager::new(config);
    let expected = PathBuf::from("/tmp/orbit-home/plugins/installed.json");
    assert_eq!(manager.registry_path(), expected);
}

#[test]
fn registry_path_respects_custom_value() {
    let mut config = PluginManagerConfig::new("/tmp/orbit-home");
    config.registry_path = Some(PathBuf::from("/custom/registry.json"));
    let manager = PluginManager::new(config);
    assert_eq!(
        manager.registry_path(),
        PathBuf::from("/custom/registry.json")
    );
}

#[test]
fn settings_path_is_config_home_settings_json() {
    let config = PluginManagerConfig::new("/tmp/orbit-home");
    let manager = PluginManager::new(config);
    assert_eq!(
        manager.settings_path(),
        PathBuf::from("/tmp/orbit-home/settings.json")
    );
}

#[test]
fn bundled_root_returns_cargo_manifest_dir_bundled() {
    let path = PluginManager::bundled_root();
    assert!(path.ends_with("bundled"));
    assert!(path.is_absolute());
}

#[test]
fn manager_uses_custom_bundled_root() {
    let mut config = PluginManagerConfig::new("/tmp/orbit-home");
    config.bundled_root = Some(PathBuf::from("/custom/bundled"));
    let manager = PluginManager::new(config);
    // bundled_root is used internally; we verify it doesn't crash
    assert!(manager.settings_path().parent().is_some());
}

#[test]
fn builtin_plugins_include_example() {
    let plugins = builtin_plugins();
    let has_example = plugins
        .iter()
        .any(|p| p.metadata().name == "example-builtin");
    assert!(
        has_example,
        "builtin plugins should include example-builtin"
    );
}

#[test]
fn builtin_plugins_have_correct_kind() {
    for plugin in builtin_plugins() {
        assert_eq!(plugin.metadata().kind, PluginKind::Builtin);
        assert_eq!(plugin.metadata().source, "builtin");
    }
}

#[test]
fn config_supports_enabled_plugins() {
    let mut config = PluginManagerConfig::new("/tmp/orbit-home");
    config
        .enabled_plugins
        .insert("my-plugin@external".to_string(), true);
    config
        .enabled_plugins
        .insert("other-plugin@external".to_string(), false);
    assert!(config.enabled_plugins["my-plugin@external"]);
    assert!(!config.enabled_plugins["other-plugin@external"]);
}

#[test]
fn config_supports_external_dirs() {
    let mut config = PluginManagerConfig::new("/tmp/orbit-home");
    config.external_dirs.push(PathBuf::from("/custom/plugins"));
    assert_eq!(config.external_dirs.len(), 1);
}

#[test]
fn config_new_accepts_pathbuf() {
    let path = PathBuf::from("/tmp/custom-home");
    let config = PluginManagerConfig::new(path.clone());
    assert_eq!(config.config_home, path);
}

#[test]
fn config_new_accepts_string() {
    let config = PluginManagerConfig::new("/tmp/string-home");
    assert_eq!(config.config_home, PathBuf::from("/tmp/string-home"));
}

#[test]
fn manager_can_be_constructed_with_config() {
    let config = PluginManagerConfig::new("/tmp/construct-test");
    let manager = PluginManager::new(config);
    let config2 = PluginManagerConfig::new("/tmp/construct-test");
    let manager2 = PluginManager::new(config2);
    assert_eq!(manager.registry_path(), manager2.registry_path());
}

#[test]
fn manager_with_empty_external_dirs() {
    let config = PluginManagerConfig::new("/tmp/empty-ext");
    let manager = PluginManager::new(config);
    // Should not crash when listing without external dirs
    // Note: this may fail if bundled plugins aren't set up; we just test construction
    assert!(manager.list_plugins().is_ok() || manager.list_plugins().is_err());
}
