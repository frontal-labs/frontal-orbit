#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // TODO: feed `data` into the parser/codec under test.
    // Example: if let Ok(s) = std::str::from_utf8(data) { let _ = orbit_x::parse(s); }
    let _ = data;
});
