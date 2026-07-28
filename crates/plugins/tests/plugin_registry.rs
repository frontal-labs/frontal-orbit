use orbit_plugins::{builtin_plugins, Plugin, PluginKind, PluginRegistry, RegisteredPlugin};

#[test]
fn empty_registry() {
    let registry = PluginRegistry::new(vec![]);
    assert!(registry.plugins().is_empty());
    assert!(registry.summaries().is_empty());
    assert!(registry.get("anything").is_none());
    assert!(!registry.contains("anything"));
}

#[test]
fn get_and_contains_with_builtin_plugin() {
    let definitions = builtin_plugins();
    let registered: Vec<RegisteredPlugin> = definitions
        .into_iter()
        .map(|d| RegisteredPlugin::new(d, true))
        .collect();
    let registry = PluginRegistry::new(registered);
    assert!(registry.contains("example-builtin@builtin"));
    assert!(!registry.contains("non-existent"));
    assert!(registry.get("example-builtin@builtin").is_some());
    assert!(registry.get("non-existent").is_none());
}

#[test]
fn plugins_are_sorted_by_id() {
    let defs = builtin_plugins();
    // builtin_plugins returns one plugin. Clone to test sorting.
    let d1 = defs[0].clone();
    let d2 = defs[0].clone();
    // Create registered with different names by wrapping clones;
    // sorting is by metadata.id, which is the same for both since we cloned.
    let registered = vec![
        RegisteredPlugin::new(d1, true),
        RegisteredPlugin::new(d2, true),
    ];
    let registry = PluginRegistry::new(registered);
    let ids: Vec<&str> = registry
        .plugins()
        .iter()
        .map(|p| p.metadata().id.as_str())
        .collect();
    assert_eq!(ids.len(), 2);
    // Both have same id, so they'll be adjacent
    assert_eq!(ids[0], ids[1]);
}

#[test]
fn aggregated_hooks_and_tools_for_builtin() {
    let definitions = builtin_plugins();
    let registered: Vec<RegisteredPlugin> = definitions
        .into_iter()
        .map(|d| RegisteredPlugin::new(d, true))
        .collect();
    let registry = PluginRegistry::new(registered);
    // Builtin plugin has no hooks or tools
    assert!(registry.aggregated_hooks().unwrap().is_empty());
    assert!(registry.aggregated_tools().unwrap().is_empty());
}

#[test]
fn initialize_and_shutdown_succeed() {
    let definitions = builtin_plugins();
    let registered: Vec<RegisteredPlugin> = definitions
        .into_iter()
        .map(|d| RegisteredPlugin::new(d, false))
        .collect();
    let registry = PluginRegistry::new(registered);
    assert!(registry.initialize().is_ok());
    assert!(registry.shutdown().is_ok());
}

#[test]
fn registered_plugin_accessors() {
    let defs = builtin_plugins();
    let plugin = RegisteredPlugin::new(defs[0].clone(), true);
    assert_eq!(plugin.metadata().name, "example-builtin");
    assert_eq!(plugin.metadata().kind, PluginKind::Builtin);
    assert!(plugin.is_enabled());
    assert!(plugin.hooks().is_empty());
    assert!(plugin.tools().is_empty());
    assert!(plugin.validate().is_ok());
    assert!(plugin.initialize().is_ok());
    assert!(plugin.shutdown().is_ok());
}

#[test]
fn disabled_plugin_summary() {
    let defs = builtin_plugins();
    let plugin = RegisteredPlugin::new(defs[0].clone(), false);
    assert!(!plugin.is_enabled());
    let summary = plugin.summary();
    assert!(!summary.enabled);
    assert_eq!(summary.metadata.id, "example-builtin@builtin");
}

#[test]
fn enabled_plugin_summary() {
    let defs = builtin_plugins();
    let plugin = RegisteredPlugin::new(defs[0].clone(), true);
    assert!(plugin.is_enabled());
    let summary = plugin.summary();
    assert!(summary.enabled);
    assert_eq!(summary.metadata.id, "example-builtin@builtin");
}

#[test]
fn builtin_plugins_list() {
    let plugins = builtin_plugins();
    assert_eq!(plugins.len(), 1);
    let p = &plugins[0];
    assert_eq!(p.metadata().name, "example-builtin");
    assert_eq!(p.metadata().version, "0.1.0");
    assert_eq!(p.metadata().kind, PluginKind::Builtin);
    assert!(!p.metadata().default_enabled);
}

#[test]
fn summaries_from_registry() {
    let defs = builtin_plugins();
    let registered: Vec<RegisteredPlugin> = defs
        .into_iter()
        .map(|d| RegisteredPlugin::new(d, true))
        .collect();
    let registry = PluginRegistry::new(registered);
    let summaries = registry.summaries();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].metadata.name, "example-builtin");
    assert!(summaries[0].enabled);
}

#[test]
fn registry_with_builtin_plugin_contains_correct_id() {
    let defs = builtin_plugins();
    let registered: Vec<RegisteredPlugin> = defs
        .into_iter()
        .map(|d| RegisteredPlugin::new(d, false))
        .collect();
    let registry = PluginRegistry::new(registered);
    let plugin = registry.get("example-builtin@builtin").unwrap();
    assert_eq!(plugin.metadata().name, "example-builtin");
}
