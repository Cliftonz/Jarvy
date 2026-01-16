# PRD-015: Config Validation and Remote Loading

## Overview

Add configuration validation and remote config loading to help teams share and validate jarvy.toml configurations.

## Problem Statement

1. No way to validate jarvy.toml before running setup - errors are discovered at runtime
2. No standard way to share configurations across team members
3. Each developer maintains their own jarvy.toml, leading to environment drift

## Requirements

### Functional Requirements

1. **Config validation**: Lint and validate configurations before use
2. **Remote config loading**: Load jarvy.toml from HTTP(S) URLs

### Non-Functional Requirements

1. Config validation must be fast (<100ms for local files)
2. Remote configs must be fetched securely (HTTPS only)
3. Clear error messages with suggestions for fixes

## User Stories

### US-1: Config Validation

**As a** config author
**I want to** validate my jarvy.toml before sharing
**So that** team members don't encounter errors

```bash
# Validate local config
jarvy validate

# Validate specific file
jarvy validate --config path/to/jarvy.toml

# Validate with strict mode (warnings become errors)
jarvy validate --strict

# Output:
# Validating jarvy.toml...
# [WARN] Line 5: Tool 'node' version '20' will match 20.x.x - consider pinning exact version
# [ERROR] Line 12: Unknown tool 'nodejs' - did you mean 'node'?
# [WARN] Line 18: Hook references unknown tool 'rg' - did you mean 'ripgrep'?
#
# Validation failed: 1 error, 2 warnings
```

**Validation checks:**
- Syntax errors in TOML
- Unknown tool names (with suggestions for typos)
- Invalid version strings
- Hooks referencing unknown tools
- Deprecated configuration options
- Missing required fields

### US-2: Remote Config Loading

**As a** team lead
**I want to** host our team's jarvy.toml on our company server
**So that** all team members use the same environment configuration

```bash
# Load config from URL
jarvy setup --from https://company.com/configs/jarvy.toml

# Load config from GitHub raw URL
jarvy setup --from https://raw.githubusercontent.com/org/repo/main/jarvy.toml

# Load config from private URL with auth
jarvy setup --from https://company.com/configs/jarvy.toml --header "Authorization: Bearer $TOKEN"

# Validate remote config without installing
jarvy validate --from https://company.com/configs/jarvy.toml
```

## Technical Approach

### Config Validation

```rust
// src/validate/mod.rs
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

pub struct ValidationError {
    pub line: Option<usize>,
    pub message: String,
    pub suggestion: Option<String>,
}

pub fn validate_config(content: &str) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Parse TOML
    let config: JarvyConfig = match toml::from_str(content) {
        Ok(c) => c,
        Err(e) => {
            result.errors.push(ValidationError {
                line: e.line(),
                message: format!("TOML syntax error: {}", e),
                suggestion: None,
            });
            return result;
        }
    };

    // Validate tools
    for (name, _spec) in &config.tools {
        if !is_known_tool(name) {
            let suggestion = find_similar_tool(name);
            result.errors.push(ValidationError {
                line: None,
                message: format!("Unknown tool '{}'", name),
                suggestion,
            });
        }
    }

    // Validate hooks reference known tools
    for (tool_name, _hook) in &config.hooks {
        if !config.tools.contains_key(tool_name) && !is_known_tool(tool_name) {
            result.warnings.push(ValidationWarning {
                line: None,
                message: format!("Hook for '{}' but tool not in [tools]", tool_name),
            });
        }
    }

    result
}
```

### Remote Config Fetching

```rust
// src/remote/mod.rs
pub struct RemoteConfig {
    pub url: Url,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
}

impl RemoteConfig {
    pub async fn fetch(&self) -> Result<String, RemoteError> {
        // Validate URL scheme (HTTPS only in production)
        if !self.url.scheme().starts_with("https") && !cfg!(debug_assertions) {
            return Err(RemoteError::InsecureUrl(self.url.clone()));
        }

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()?;

        let mut request = client.get(self.url.clone());
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(RemoteError::HttpError(response.status()));
        }

        let content = response.text().await?;

        // Validate before returning
        validate_config(&content)?;

        Ok(content)
    }
}
```

## Security Considerations

### Remote Config Security

1. **HTTPS Only**: Remote configs must be loaded over HTTPS (HTTP allowed only in debug mode)
2. **Content Validation**: Validate TOML syntax and schema before executing any commands
3. **No Arbitrary Code**: Config files cannot execute arbitrary code; only declarative tool specifications
4. **Certificate Validation**: Use system certificate store, require valid TLS certificates
5. **Timeout Protection**: Enforce timeouts to prevent hanging on slow/malicious servers
6. **Size Limits**: Limit config file size (e.g., 1MB) to prevent memory exhaustion

## CLI Commands

```bash
# Validation
jarvy validate                       # Validate local config
jarvy validate --config FILE         # Validate specific file
jarvy validate --from URL            # Validate remote config
jarvy validate --strict              # Warnings become errors

# Remote config
jarvy setup --from URL               # Load and run remote config
jarvy setup --from URL --header "K: V"  # With auth header
```

## Implementation Steps

1. Create `src/validate/mod.rs` module for config validation
2. Implement known-tools registry check
3. Add fuzzy matching for tool name suggestions (strsim crate or simple Levenshtein)
4. Add `jarvy validate` CLI command
5. Create `src/remote/mod.rs` module for remote config fetching
6. Add reqwest dependency with minimal features
7. Implement `--from` flag on `jarvy setup`
8. Add `--from` flag on `jarvy validate`
9. Write tests for validation rules
10. Write tests for remote fetching
11. Update documentation

## Acceptance Criteria

1. **Config Validation**
   - `jarvy validate` checks local jarvy.toml
   - Unknown tool names produce errors with suggestions
   - Invalid version strings produce errors
   - TOML syntax errors show line numbers
   - Exit code 0 for valid, non-zero for errors
   - `--strict` treats warnings as errors

2. **Remote Config Loading**
   - `jarvy setup --from <URL>` fetches and uses remote config
   - HTTPS URLs work with valid certificates
   - HTTP URLs are rejected in release builds
   - Custom headers can be passed for authentication
   - Clear error messages for network failures
   - `jarvy validate --from <URL>` validates without installing

## Non-Goals

1. **Config caching**: Not caching remote configs (fetch fresh each time)
2. **Config inheritance**: No `extends` field for now
3. **Profiles**: No tool subsets/profiles
4. **Lock files**: No version pinning beyond what's in jarvy.toml
5. **Audit logging**: No change tracking

## Dependencies

- `reqwest` - HTTP client (minimal features: rustls-tls)
- `strsim` - String similarity for suggestions (optional, could use simple algorithm)

## Files to Create/Modify

### New Files
- `src/validate/mod.rs` - Config validation logic
- `src/remote/mod.rs` - Remote config fetching
- `tests/validate.rs` - Validation tests
- `tests/remote_config.rs` - Remote config tests

### Modified Files
- `src/main.rs` - Add validate command and --from flag
- `src/config.rs` - Expose validation hooks
- `Cargo.toml` - Add reqwest dependency
