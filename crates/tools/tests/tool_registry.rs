use orbit_tools::{ToolManifestEntry, ToolRegistry, ToolSource};

#[test]
fn tool_registry_empty() {
    let registry = ToolRegistry::new(vec![]);
    assert!(registry.entries().is_empty());
}

#[test]
fn tool_registry_with_entries() {
    let entries = vec![
        ToolManifestEntry {
            name: "BashTool".to_string(),
            source: ToolSource::Base,
        },
        ToolManifestEntry {
            name: "ReadTool".to_string(),
            source: ToolSource::Base,
        },
    ];
    let registry = ToolRegistry::new(entries);
    assert_eq!(registry.entries().len(), 2);
}

#[test]
fn tool_manifest_entry_display() {
    let entry = ToolManifestEntry {
        name: "BashTool".to_string(),
        source: ToolSource::Base,
    };
    assert_eq!(entry.name, "BashTool");
    assert_eq!(entry.source, ToolSource::Base);
}

#[test]
fn tool_source_variants() {
    assert_eq!(format!("{:?}", ToolSource::Base), "Base");
    assert_eq!(format!("{:?}", ToolSource::Conditional), "Conditional");
}

#[test]
fn tool_registry_maintains_insertion_order() {
    let entries = vec![
        ToolManifestEntry {
            name: "WriteTool".to_string(),
            source: ToolSource::Base,
        },
        ToolManifestEntry {
            name: "BashTool".to_string(),
            source: ToolSource::Base,
        },
    ];
    let registry = ToolRegistry::new(entries);
    let names: Vec<&str> = registry.entries().iter().map(|e| e.name.as_str()).collect();
    // Registry preserves insertion order
    assert_eq!(names, vec!["WriteTool", "BashTool"]);
}
