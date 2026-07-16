//! Conversion of SDK config objects into `orbit` CLI `--config key=value`
//! arguments.

use serde_json::Value;

/// Serialize a JSON value as a TOML literal, suitable for a `--config key=value`
/// CLI flag. Strings are emitted as TOML basic strings (double-quoted).
pub fn to_toml_literal(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s)
            .unwrap_or_else(|_| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))),
        Value::Array(items) => {
            let serialized: Vec<String> = items.iter().map(to_toml_literal).collect();
            format!("[{}]", serialized.join(", "))
        }
        Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(key, nested)| format!("{key} = {}", to_toml_literal(nested)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
    }
}

/// Flatten a JSON object into dotted-path leaves. Nested objects become
/// `parent.child.key`; arrays and primitives are treated as leaves.
pub fn flatten_config(config: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk(String::new(), config, &mut out);
    out
}

fn walk(prefix: String, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                walk(next, &map[key], out);
            }
        }
        _ => out.push((prefix, to_toml_literal(value))),
    }
}

/// Convert a config value into repeated `--config key=value` CLI arguments.
pub fn config_to_args(config: &Value) -> Vec<String> {
    let Value::Object(map) = config else {
        return Vec::new();
    };
    if map.is_empty() {
        return Vec::new();
    }
    let mut args = Vec::new();
    for (key, literal) in flatten_config(config) {
        args.push("--config".to_string());
        args.push(format!("{key}={literal}"));
    }
    args
}

/// Deep-merge two JSON configs. Object values are merged recursively; all
/// other values (and `overlay` when its counterpart is not an object) replace
/// the base. Used to layer global, thread, and run-level config overrides.
pub fn merge_config(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            let mut merged = base_map.clone();
            for (key, overlay_value) in overlay_map {
                let next = match merged.get(key) {
                    Some(existing) => merge_config(existing, overlay_value),
                    None => overlay_value.clone(),
                };
                merged.insert(key.clone(), next);
            }
            Value::Object(merged)
        }
        _ => overlay.clone(),
    }
}
