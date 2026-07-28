use orbit_commands::{CommandManifestEntry, CommandRegistry, CommandSource};

#[test]
fn command_registry_empty_by_default() {
    let registry = CommandRegistry::new(vec![]);
    assert!(registry.entries().is_empty());
}

#[test]
fn command_registry_with_entries() {
    let entries = vec![
        CommandManifestEntry {
            name: "help".to_string(),
            source: CommandSource::Builtin,
        },
        CommandManifestEntry {
            name: "status".to_string(),
            source: CommandSource::Builtin,
        },
    ];
    let registry = CommandRegistry::new(entries.clone());
    assert_eq!(registry.entries(), &entries);
}

#[test]
fn command_manifest_entry_all_sources() {
    let builtin = CommandManifestEntry {
        name: "help".to_string(),
        source: CommandSource::Builtin,
    };
    let internal = CommandManifestEntry {
        name: "secret".to_string(),
        source: CommandSource::InternalOnly,
    };
    let gated = CommandManifestEntry {
        name: "beta".to_string(),
        source: CommandSource::FeatureGated,
    };
    assert_eq!(builtin.name, "help");
    assert_eq!(builtin.source, CommandSource::Builtin);
    assert_eq!(internal.source, CommandSource::InternalOnly);
    assert_eq!(gated.source, CommandSource::FeatureGated);
}

#[test]
fn command_source_debug_and_clone() {
    let sources = [
        CommandSource::Builtin,
        CommandSource::InternalOnly,
        CommandSource::FeatureGated,
    ];
    for source in sources {
        let cloned = source;
        assert_eq!(source, cloned);
        let debug = format!("{source:?}");
        assert!(!debug.is_empty());
    }
}

#[test]
fn command_source_partial_eq() {
    assert_eq!(CommandSource::Builtin, CommandSource::Builtin);
    assert_ne!(CommandSource::Builtin, CommandSource::InternalOnly);
    assert_ne!(CommandSource::Builtin, CommandSource::FeatureGated);
}

#[test]
fn command_registry_supports_multiple_entries() {
    let entries = (0..10)
        .map(|i| CommandManifestEntry {
            name: format!("cmd{i}"),
            source: CommandSource::Builtin,
        })
        .collect::<Vec<_>>();
    let registry = CommandRegistry::new(entries);
    assert_eq!(registry.entries().len(), 10);
}

#[test]
fn command_manifest_entry_debug() {
    let entry = CommandManifestEntry {
        name: "test".to_string(),
        source: CommandSource::Builtin,
    };
    let debug = format!("{entry:?}");
    assert!(debug.contains("test"));
    assert!(debug.contains("Builtin"));
}

#[test]
fn command_registry_default_is_empty() {
    let registry = CommandRegistry::default();
    assert_eq!(registry.entries().len(), 0);
}
