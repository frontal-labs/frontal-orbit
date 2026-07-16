#![no_main]
use libfuzzer_sys::fuzz_target;

/// Smoke fuzz target: exercises UTF-8 / JSON-ish parsing paths that the
/// `providers` and `tools` crates rely on. Replace the body with a real
/// parser under test once wired to a crate dependency.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<serde_json::Value>(s);
    }
});
