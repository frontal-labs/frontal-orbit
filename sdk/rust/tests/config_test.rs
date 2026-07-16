use orbit_sdk::{config_to_args, flatten_config, merge_config, to_toml_literal};
use serde_json::json;

#[test]
fn to_toml_literal_primitives() {
    assert_eq!(to_toml_literal(&json!(true)), "true");
    assert_eq!(to_toml_literal(&json!(false)), "false");
    assert_eq!(to_toml_literal(&json!(42)), "42");
    assert_eq!(to_toml_literal(&json!(3.5)), "3.5");
    assert_eq!(to_toml_literal(&json!("hello")), "\"hello\"");
    assert_eq!(to_toml_literal(&json!(null)), "null");
}

#[test]
fn to_toml_literal_collections() {
    assert_eq!(to_toml_literal(&json!([1, 2, 3])), "[1, 2, 3]");
    assert_eq!(
        to_toml_literal(&json!({ "network_access": true })),
        "{ network_access = true }"
    );
    assert_eq!(
        to_toml_literal(&json!({ "a": 1, "b": { "c": "x" } })),
        "{ a = 1, b = { c = \"x\" } }"
    );
}

#[test]
fn flatten_config_dotted_paths() {
    let flat = flatten_config(&json!({
        "show_raw_agent_reasoning": true,
        "sandbox_workspace_write": { "network_access": true },
    }));
    assert_eq!(
        flat,
        vec![
            (
                "sandbox_workspace_write.network_access".to_string(),
                "true".to_string()
            ),
            ("show_raw_agent_reasoning".to_string(), "true".to_string()),
        ]
    );
}

#[test]
fn flatten_config_arrays_are_leaves() {
    let flat = flatten_config(&json!({ "tags": ["a", "b"] }));
    assert_eq!(
        flat,
        vec![("tags".to_string(), "[\"a\", \"b\"]".to_string())]
    );
}

#[test]
fn config_to_args_repeated_flags() {
    let args = config_to_args(&json!({
        "show_raw_agent_reasoning": true,
        "sandbox_workspace_write": { "network_access": true },
    }));
    assert_eq!(
        args,
        vec![
            "--config".to_string(),
            "sandbox_workspace_write.network_access=true".to_string(),
            "--config".to_string(),
            "show_raw_agent_reasoning=true".to_string(),
        ]
    );
}

#[test]
fn config_to_args_empty_for_null() {
    assert!(config_to_args(&json!(null)).is_empty());
}

#[test]
fn merge_config_overlays_objects() {
    let base = json!({ "a": 1, "nested": { "x": 1, "y": 2 } });
    let overlay = json!({ "b": 2, "nested": { "y": 20, "z": 30 } });
    let merged = merge_config(&base, &overlay);
    assert_eq!(
        merged,
        json!({ "a": 1, "b": 2, "nested": { "x": 1, "y": 20, "z": 30 } })
    );
}

#[test]
fn merge_config_replaces_scalars() {
    let merged = merge_config(&json!({ "a": 1 }), &json!({ "a": 2 }));
    assert_eq!(merged, json!({ "a": 2 }));
}
