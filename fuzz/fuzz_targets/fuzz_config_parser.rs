//! Fuzz target for jarvy.toml config parsing
//!
//! Tests that the TOML parser handles arbitrary input without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to interpret bytes as UTF-8 string
    if let Ok(s) = std::str::from_utf8(data) {
        // Attempt to parse as TOML
        let _ = toml::from_str::<toml::Value>(s);
    }
});
