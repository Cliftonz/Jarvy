//! Property-based tests for version parsing and comparison
//!
//! Properties tested:
//! 1. Version ordering is transitive
//! 2. Version parsing is deterministic
//! 3. Version requirements match correctly
//! 4. "latest" handling is consistent

use proptest::prelude::*;
use semver::{Version, VersionReq};

/// Generate a valid semver version
fn semver_strategy() -> impl Strategy<Value = Version> {
    (0u64..1000, 0u64..1000, 0u64..1000)
        .prop_map(|(major, minor, patch)| Version::new(major, minor, patch))
}

/// Generate a valid version requirement
fn version_req_strategy() -> impl Strategy<Value = VersionReq> {
    prop_oneof![
        // Exact version
        semver_strategy().prop_map(|v| VersionReq::parse(&format!("={}", v)).unwrap()),
        // Caret requirement
        semver_strategy().prop_map(|v| VersionReq::parse(&format!("^{}", v)).unwrap()),
        // Tilde requirement
        semver_strategy().prop_map(|v| VersionReq::parse(&format!("~{}", v)).unwrap()),
        // Greater than or equal
        semver_strategy().prop_map(|v| VersionReq::parse(&format!(">={}", v)).unwrap()),
        // Less than
        semver_strategy().prop_map(|v| VersionReq::parse(&format!("<{}", v)).unwrap()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: Version comparison is reflexive (v == v)
    #[test]
    fn prop_version_reflexive(v in semver_strategy()) {
        prop_assert_eq!(&v, &v);
        prop_assert!(v.cmp(&v) == std::cmp::Ordering::Equal);
    }

    /// Property: Version comparison is antisymmetric
    #[test]
    fn prop_version_antisymmetric(v1 in semver_strategy(), v2 in semver_strategy()) {
        if v1 < v2 {
            prop_assert!(!(v2 < v1));
            prop_assert!(v2 > v1);
        } else if v1 > v2 {
            prop_assert!(!(v2 > v1));
            prop_assert!(v2 < v1);
        } else {
            prop_assert_eq!(v1, v2);
        }
    }

    /// Property: Version comparison is transitive
    #[test]
    fn prop_version_transitive(
        v1 in semver_strategy(),
        v2 in semver_strategy(),
        v3 in semver_strategy()
    ) {
        if v1 < v2 && v2 < v3 {
            prop_assert!(v1 < v3);
        }
        if v1 > v2 && v2 > v3 {
            prop_assert!(v1 > v3);
        }
        if v1 == v2 && v2 == v3 {
            prop_assert_eq!(v1, v3);
        }
    }

    /// Property: Version parsing is deterministic
    #[test]
    fn prop_version_parsing_deterministic(
        major in 0u64..1000,
        minor in 0u64..1000,
        patch in 0u64..1000
    ) {
        let version_str = format!("{}.{}.{}", major, minor, patch);
        let v1 = Version::parse(&version_str).unwrap();
        let v2 = Version::parse(&version_str).unwrap();
        prop_assert_eq!(v1, v2);
    }

    /// Property: Version round-trips through string
    #[test]
    fn prop_version_roundtrip(v in semver_strategy()) {
        let s = v.to_string();
        let v2 = Version::parse(&s).unwrap();
        prop_assert_eq!(v, v2);
    }

    /// Property: Caret requirements match expected versions
    #[test]
    fn prop_caret_matching(
        major in 1u64..100,
        minor in 0u64..100,
        patch in 0u64..100
    ) {
        let base = Version::new(major, minor, patch);
        let req = VersionReq::parse(&format!("^{}", base)).unwrap();

        // Same version should always match
        prop_assert!(req.matches(&base));

        // Higher patch should match
        let higher_patch = Version::new(major, minor, patch + 1);
        prop_assert!(req.matches(&higher_patch));

        // Higher minor should match (for major > 0)
        if major > 0 {
            let higher_minor = Version::new(major, minor + 1, 0);
            prop_assert!(req.matches(&higher_minor));
        }

        // Higher major should NOT match
        let higher_major = Version::new(major + 1, 0, 0);
        prop_assert!(!req.matches(&higher_major));
    }

    /// Property: Exact version requirements only match exact versions
    #[test]
    fn prop_exact_matching(v in semver_strategy()) {
        let req = VersionReq::parse(&format!("={}", v)).unwrap();

        // Exact match
        prop_assert!(req.matches(&v));

        // Different patch should not match
        let different = Version::new(v.major, v.minor, v.patch.saturating_add(1));
        if different != v {
            prop_assert!(!req.matches(&different));
        }
    }

    /// Property: Version requirement parsing is deterministic
    #[test]
    fn prop_req_parsing_deterministic(req in version_req_strategy()) {
        let s = req.to_string();
        let req2 = VersionReq::parse(&s).unwrap();
        // Note: string representation may differ but matching behavior should be same
        // We verify by testing against the same version
        let test_version = Version::new(1, 2, 3);
        prop_assert_eq!(req.matches(&test_version), req2.matches(&test_version));
    }

    /// Property: Higher versions are greater
    #[test]
    fn prop_version_ordering(
        major in 0u64..100,
        minor in 0u64..100,
        patch in 0u64..100
    ) {
        let v = Version::new(major, minor, patch);

        // Incrementing any component makes version larger
        let higher_patch = Version::new(major, minor, patch + 1);
        prop_assert!(higher_patch > v);

        let higher_minor = Version::new(major, minor + 1, patch);
        prop_assert!(higher_minor > v);

        let higher_major = Version::new(major + 1, minor, patch);
        prop_assert!(higher_major > v);

        // Major dominates minor, minor dominates patch
        let higher_major_lower_minor = Version::new(major + 1, 0, 0);
        let lower_major_higher_minor = Version::new(major, minor + 100, patch + 100);
        prop_assert!(higher_major_lower_minor > lower_major_higher_minor);
    }
}

/// Test "latest" version string handling
#[test]
fn test_latest_version_handling() {
    let versions = ["latest", "LATEST", "Latest", "lAtEsT"];
    for v in versions {
        assert!(v.eq_ignore_ascii_case("latest"));
    }

    let not_latest = ["last", "newest", "1.0.0", ""];
    for v in not_latest {
        assert!(!v.eq_ignore_ascii_case("latest"));
    }
}

/// Test version parsing edge cases
#[test]
fn test_version_parsing_edge_cases() {
    // Valid versions
    assert!(Version::parse("0.0.0").is_ok());
    assert!(Version::parse("0.0.1").is_ok());
    assert!(Version::parse("999.999.999").is_ok());

    // Invalid versions
    assert!(Version::parse("").is_err());
    assert!(Version::parse("1").is_err());
    assert!(Version::parse("1.2").is_err());
    assert!(Version::parse("a.b.c").is_err());
    assert!(Version::parse("-1.0.0").is_err());
}
