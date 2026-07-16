//! Minimal, dependency-free templating used by `tools-generators` and
//! `tools-codegen`. Templates use `{name}` placeholders; unknown placeholders
//! are left intact so partially-applied templates remain readable.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::Path;

/// Render `template` by substituting `{key}` occurrences from `vars`.
///
/// Unknown placeholders are preserved verbatim so a template can be applied
/// incrementally. `{{` and `}}` are treated as literal braces (escaping).
pub fn render<S: BuildHasher>(template: &str, vars: &HashMap<&str, String, S>) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            out.push('{');
            i += 2;
            continue;
        }
        if bytes[i] == b'}' && i + 1 < bytes.len() && bytes[i + 1] == b'}' {
            out.push('}');
            i += 2;
            continue;
        }
        if bytes[i] == b'{' {
            let start = i + 1;
            if let Some(end) = template[start..].find('}') {
                let key = &template[start..start + end];
                if let Some(v) = vars.get(key) {
                    out.push_str(v);
                } else {
                    out.push('{');
                    out.push_str(key);
                    out.push('}');
                }
                i = start + end + 1;
                continue;
            }
        }
        // Safe because we only ever push valid UTF-8 boundaries (we scanned
        // byte-by-byte but only skip on ASCII delimiters '{' '}').
        let ch = template[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Load `name` from `dir` and render it with `vars`.
pub fn render_file<S: BuildHasher>(dir: &Path, name: &str, vars: &HashMap<&str, String, S>) -> std::io::Result<String> {
    let content = std::fs::read_to_string(dir.join(name))?;
    Ok(render(&content, vars))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_vars() {
        let mut v = HashMap::new();
        v.insert("name", "orbit".to_string());
        v.insert("year", "2026".to_string());
        assert_eq!(render("hi {name} in {year}", &v), "hi orbit in 2026");
    }

    #[test]
    fn leaves_unknown_vars_intact() {
        assert_eq!(render("a {missing} b", &HashMap::new()), "a {missing} b");
    }

    #[test]
    fn escapes_double_braces() {
        assert_eq!(render("literal {{x}}", &HashMap::new()), "literal {x}");
    }
}
