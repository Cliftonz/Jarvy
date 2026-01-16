//! Fuzz target for version string parsing
//!
//! Tests that semver parsing handles arbitrary input without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz semver::Version parsing
        let _ = semver::Version::parse(s);

        // Fuzz semver::VersionReq parsing
        let _ = semver::VersionReq::parse(s);

        // Fuzz "latest" detection
        let _ = s.eq_ignore_ascii_case("latest");
    }
});
