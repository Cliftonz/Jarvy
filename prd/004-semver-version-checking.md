# PRD-004: Semantic Version Checking

## Overview

Replace the current substring-based version matching with proper semantic versioning support, enabling version ranges and accurate version comparison.

## Problem Statement

Current version checking in `cmd_satisfies()` uses simple substring matching:

```rust
pub fn cmd_satisfies(cmd: &str, min_prefix: &str) -> bool {
    if let Ok(out) = Command::new(cmd).arg("--version").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        return s.contains(min_prefix);  // BUG: Substring match!
    }
    false
}
```

This causes:
- **False positives**: `"2.4"` matches `"12.40"`, `"2.401"`, `"v2.4.0-beta"`
- **False negatives**: `"2.40"` doesn't match `"2.40.1"` if user expects it to
- **No range support**: Can't specify `">= 3.10, < 4.0"`

## Evidence

- `cmd_satisfies("git", "2.4")` returns TRUE if git version is "1.24" (contains "2.4")
- User specifies `python = "3.10"` expecting 3.10+, but 3.9 with substring "3.1" might pass
- No tests exist for version matching logic

## Requirements

### Functional Requirements

1. **Exact version matching**: `"3.10.0"` matches only 3.10.0
2. **Prefix matching**: `"3.10"` matches 3.10.x
3. **Range expressions**: `">= 3.10"`, `"< 4.0"`, `">= 3.10, < 4.0"`
4. **Wildcards**: `"3.x"`, `"3.10.x"`, `"*"` (any version)
5. **Latest keyword**: `"latest"` always passes (skip check)
6. **Prerelease handling**: `"3.10.0-beta"` recognized but not matched by `"3.10"`

### Non-Functional Requirements

1. Parse version output from any tool format
2. Handle missing version components (e.g., "3" → "3.0.0")
3. Support non-semver versions gracefully (fall back to prefix match)
4. Zero performance regression on version checks

## Version Output Formats to Handle

| Tool | Version Output | Extracted Version |
|------|---------------|-------------------|
| git | `git version 2.44.0` | 2.44.0 |
| node | `v20.10.0` | 20.10.0 |
| python | `Python 3.12.1` | 3.12.1 |
| docker | `Docker version 24.0.7, build afdd53b` | 24.0.7 |
| rustc | `rustc 1.75.0 (82e1608df 2023-12-21)` | 1.75.0 |
| go | `go version go1.21.5 darwin/arm64` | 1.21.5 |

## Technical Approach

### Version Extraction

```rust
use regex::Regex;

lazy_static! {
    static ref VERSION_REGEX: Regex = Regex::new(
        r"v?(\d+)\.(\d+)(?:\.(\d+))?(?:-([a-zA-Z0-9.-]+))?"
    ).unwrap();
}

pub fn extract_version(output: &str) -> Option<Version> {
    VERSION_REGEX.captures(output).map(|caps| {
        Version {
            major: caps[1].parse().unwrap(),
            minor: caps[2].parse().unwrap(),
            patch: caps.get(3).map(|m| m.as_str().parse().unwrap()).unwrap_or(0),
            prerelease: caps.get(4).map(|m| m.as_str().to_string()),
        }
    })
}
```

### Version Comparison

```rust
use semver::{Version, VersionReq};

pub fn version_satisfies(installed: &str, requirement: &str) -> bool {
    // Handle special cases
    if requirement == "latest" || requirement == "*" {
        return true;
    }

    // Try to parse as semver requirement
    if let Ok(req) = VersionReq::parse(requirement) {
        if let Some(ver) = extract_version(installed) {
            return req.matches(&ver);
        }
    }

    // Fall back to prefix matching for non-semver
    installed.contains(requirement)
}
```

### Requirement Syntax

| Syntax | Meaning | Example |
|--------|---------|---------|
| `3.10` | 3.10.x (any patch) | Matches 3.10.0, 3.10.5 |
| `3.10.0` | Exactly 3.10.0 | Only 3.10.0 |
| `>= 3.10` | 3.10 or higher | Matches 3.10, 3.11, 4.0 |
| `< 4.0` | Less than 4.0 | Matches 3.x |
| `>= 3.10, < 4.0` | Range | Matches 3.10 - 3.x |
| `~3.10` | Compatible with 3.10 | Matches 3.10.x |
| `^3.10` | Compatible with 3.x | Matches 3.10+ |
| `latest` | Skip version check | Always passes |
| `*` | Any version | Always passes |

## Config Examples

```toml
[tools]
# Prefix matching (current behavior, still supported)
git = "2.40"

# Exact version
node = "20.10.0"

# Range expression
python = ">= 3.10, < 4.0"

# Any version
docker = "*"

# Skip check
terraform = "latest"

# Complex example
[tools.rust]
version = "^1.70"
version_manager = true
```

## Implementation Steps

1. Add `semver` crate to dependencies
2. Create `src/version.rs` with extraction and comparison logic
3. Add regex patterns for common version output formats
4. Refactor `cmd_satisfies()` to use new logic
5. Add fallback to prefix matching for unrecognized formats
6. Write comprehensive unit tests for version matching
7. Update config documentation with version syntax
8. Add CLI command to test version matching: `jarvy version-check git "2.40"`

## Test Cases

```rust
#[test]
fn test_version_extraction() {
    assert_eq!(extract_version("git version 2.44.0"), Some(v(2, 44, 0)));
    assert_eq!(extract_version("v20.10.0"), Some(v(20, 10, 0)));
    assert_eq!(extract_version("Python 3.12.1"), Some(v(3, 12, 1)));
    assert_eq!(extract_version("go1.21.5"), Some(v(1, 21, 5)));
}

#[test]
fn test_version_satisfies() {
    // Prefix matching
    assert!(version_satisfies("2.44.0", "2.44"));
    assert!(!version_satisfies("2.43.0", "2.44"));

    // Range matching
    assert!(version_satisfies("3.10.5", ">= 3.10, < 4.0"));
    assert!(!version_satisfies("4.0.0", ">= 3.10, < 4.0"));

    // Wildcards
    assert!(version_satisfies("1.0.0", "*"));
    assert!(version_satisfies("99.0.0", "latest"));

    // Edge cases
    assert!(!version_satisfies("12.40.0", "2.4"));  // Fixed bug!
}
```

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| False positive rate | Unknown (high) | 0% |
| Version syntax support | Prefix only | Full semver |
| Test coverage | 0% | 95% |

## Risks

1. **Breaking change**: Some users may rely on substring behavior
   - Mitigation: Log warning when falling back to prefix matching
   - Mitigation: Add `--strict-versions` flag initially
2. **Unparseable version output**: Some tools have weird formats
   - Mitigation: Maintain per-tool version regex overrides
3. **Performance**: Regex parsing on every check
   - Mitigation: Cache parsed versions in memory

## Dependencies

- `semver` crate (well-maintained, standard)
- `regex` crate (likely already in tree)
- `lazy_static` or `once_cell` for compiled regex

## Effort Estimate

- Version extraction: 1 day
- Requirement parsing: 0.5 days
- Integration: 0.5 days
- Testing: 1 day
- Documentation: 0.5 days

## Files to Modify

- `Cargo.toml` - Add semver dependency
- `src/version.rs` - New file
- `src/tools/common.rs` - Refactor cmd_satisfies()
- `src/config.rs` - Update version field parsing
- `tests/version_matching.rs` - New test file
