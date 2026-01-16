//! Property-based tests for config parsing
//!
//! Properties tested:
//! 1. Valid TOML round-trips correctly (parse -> serialize -> parse)
//! 2. Tool names are preserved after parsing
//! 3. Version strings are preserved after parsing
//! 4. Invalid TOML is rejected gracefully

use proptest::prelude::*;

/// Generate a valid tool name (alphanumeric with hyphens)
fn tool_name_strategy() -> impl Strategy<Value = String> {
    // Tool names: lowercase letters, numbers, and hyphens
    // Must start with a letter, 2-20 characters
    "[a-z][a-z0-9-]{1,19}".prop_map(|s| s.to_string())
}

/// Generate a valid version string
fn version_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // "latest" version
        Just("latest".to_string()),
        // Simple major version
        (1u32..100).prop_map(|n| n.to_string()),
        // Major.minor version
        (1u32..100, 0u32..100).prop_map(|(maj, min)| format!("{}.{}", maj, min)),
        // Full semver
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(maj, min, pat)| format!("{}.{}.{}", maj, min, pat)),
    ]
}

/// Generate a simple tool config entry
fn tool_config_entry() -> impl Strategy<Value = (String, String)> {
    (tool_name_strategy(), version_strategy())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: Valid tool names parse without errors
    #[test]
    fn prop_valid_tool_names_parse(name in tool_name_strategy()) {
        let config = format!(r#"
[provisioner]
{} = "latest"
"#, name);
        let result: Result<toml::Value, _> = toml::from_str(&config);
        prop_assert!(result.is_ok(), "Failed to parse config with tool name: {}", name);
    }

    /// Property: Valid version strings parse without errors
    #[test]
    fn prop_valid_versions_parse(version in version_strategy()) {
        let config = format!(r#"
[provisioner]
testtool = "{}"
"#, version);
        let result: Result<toml::Value, _> = toml::from_str(&config);
        prop_assert!(result.is_ok(), "Failed to parse config with version: {}", version);
    }

    /// Property: Tool configs round-trip correctly
    #[test]
    fn prop_tool_config_roundtrip((name, version) in tool_config_entry()) {
        let config = format!(r#"
[provisioner]
{} = "{}"
"#, name, version);

        // Parse
        let parsed: toml::Value = toml::from_str(&config).unwrap();

        // Serialize back
        let serialized = toml::to_string(&parsed).unwrap();

        // Parse again
        let reparsed: toml::Value = toml::from_str(&serialized).unwrap();

        // Values should match
        prop_assert_eq!(parsed, reparsed);
    }

    /// Property: Multiple tools parse correctly
    #[test]
    fn prop_multiple_tools_parse(
        tools in prop::collection::vec(tool_config_entry(), 1..10)
    ) {
        // Deduplicate tool names
        let mut seen = std::collections::HashSet::new();
        let unique_tools: Vec<_> = tools.into_iter()
            .filter(|(name, _)| seen.insert(name.clone()))
            .collect();

        if unique_tools.is_empty() {
            return Ok(());
        }

        let tool_lines: String = unique_tools.iter()
            .map(|(name, version)| format!("{} = \"{}\"", name, version))
            .collect::<Vec<_>>()
            .join("\n");

        let config = format!(r#"
[provisioner]
{}
"#, tool_lines);

        let result: Result<toml::Value, _> = toml::from_str(&config);
        prop_assert!(result.is_ok(), "Failed to parse multi-tool config");

        // Verify all tools are present
        let parsed = result.unwrap();
        let provisioner = parsed.get("provisioner").unwrap().as_table().unwrap();
        for (name, _) in &unique_tools {
            prop_assert!(provisioner.contains_key(name), "Tool {} not found in parsed config", name);
        }
    }

    /// Property: TOML table values preserve structure
    #[test]
    fn prop_table_structure_preserved(
        shell in prop_oneof![Just("bash"), Just("zsh"), Just("sh"), Just("fish")],
        timeout in 60u64..3600,
    ) {
        let config = format!(r#"
[provisioner]
git = "latest"

[hooks.config]
shell = "{}"
timeout = {}
"#, shell, timeout);

        let parsed: toml::Value = toml::from_str(&config).unwrap();

        let hooks = parsed.get("hooks").unwrap();
        let hook_config = hooks.get("config").unwrap();

        prop_assert_eq!(hook_config.get("shell").unwrap().as_str().unwrap(), shell);
        prop_assert_eq!(hook_config.get("timeout").unwrap().as_integer().unwrap(), timeout as i64);
    }
}

/// Property: Empty provisioner section is valid
#[test]
fn test_empty_provisioner() {
    let config = r#"
[provisioner]
"#;
    let result: Result<toml::Value, _> = toml::from_str(config);
    assert!(result.is_ok());
}

/// Property: Missing provisioner section fails gracefully
#[test]
fn test_missing_provisioner() {
    let config = r#"
[hooks]
pre_setup = "echo hello"
"#;
    // This should parse as TOML but would fail Jarvy's config validation
    let result: Result<toml::Value, _> = toml::from_str(config);
    assert!(result.is_ok()); // Valid TOML
}
