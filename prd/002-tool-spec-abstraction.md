# PRD-002: Tool Specification Abstraction

## Overview

Eliminate ~80% of code duplication across 42 tool implementations by introducing a declarative `ToolSpec` pattern.

## Problem Statement

Every tool in `src/tools/{name}/{name}.rs` implements nearly identical boilerplate:

```rust
pub fn ensure(min_hint: &str) -> Result<(), InstallError> {
    if cmd_satisfies("command_name", min_hint) {
        return Ok(());
    }
    install()
}

pub fn add_handler(min_hint: &str) -> Result<(), InstallError> {
    ensure(min_hint)
}

fn install() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")] { return install_macos(); }
    #[cfg(target_os = "linux")] { return install_linux(); }
    #[cfg(target_os = "windows")] { return install_windows(); }
    Err(InstallError::Unsupported)
}
```

This pattern is repeated 42 times, resulting in ~4,200 lines that could be ~500.

## Evidence

- 42 tool files with identical structure
- Package manager mappings repeated 33+ times
- Each new tool requires 50-80 lines of copy-paste
- Bug fixes require updates across all tool files

## Requirements

### Functional Requirements

1. **Declarative tool definition**: Define tools via data, not code
2. **Package name mapping**: Central registry of PM-specific package names
3. **Platform support flags**: Declare which platforms a tool supports
4. **Custom install hooks**: Allow override for complex tools (nvm, rustup)
5. **Auto-registration**: Tools register themselves without manual mod.rs edits

### Non-Functional Requirements

1. Zero runtime overhead vs current approach
2. Compile-time validation of tool specs
3. IDE autocomplete for tool fields
4. Backward compatible with existing tool implementations

## Technical Approach

### Option A: Struct-based Definition (Recommended)

```rust
// src/tools/spec.rs
pub struct ToolSpec {
    pub name: &'static str,
    pub command: &'static str,
    pub version_flag: &'static str,  // usually "--version"
    pub macos: Option<MacOsInstall>,
    pub linux: Option<LinuxInstall>,
    pub windows: Option<WindowsInstall>,
}

pub struct MacOsInstall {
    pub homebrew: Option<&'static str>,  // formula name
    pub cask: Option<&'static str>,      // cask name
}

pub struct LinuxInstall {
    pub apt: Option<&'static str>,
    pub dnf: Option<&'static str>,
    pub pacman: Option<&'static str>,
    pub apk: Option<&'static str>,
}

pub struct WindowsInstall {
    pub winget: Option<&'static str>,
    pub choco: Option<&'static str>,
}

// Usage in src/tools/git.rs
pub static GIT: ToolSpec = ToolSpec {
    name: "git",
    command: "git",
    version_flag: "--version",
    macos: Some(MacOsInstall { homebrew: Some("git"), cask: None }),
    linux: Some(LinuxInstall { apt: Some("git"), dnf: Some("git"), pacman: Some("git"), apk: Some("git") }),
    windows: Some(WindowsInstall { winget: Some("Git.Git"), choco: Some("git") }),
};
```

**Pros**: Type-safe, IDE support, compile-time validation
**Cons**: Verbose for simple tools

### Option B: Macro-based Definition

```rust
tool! {
    name: "git",
    command: "git",
    macos: { brew: "git" },
    linux: { apt: "git", dnf: "git", pacman: "git" },
    windows: { winget: "Git.Git" },
}
```

**Pros**: Concise, less boilerplate
**Cons**: Harder to debug, IDE support varies

### Option C: TOML/YAML Definitions

```toml
# tools/git.toml
name = "git"
command = "git"

[macos]
homebrew = "git"

[linux]
apt = "git"
dnf = "git"
```

**Pros**: Non-Rust contributors can add tools
**Cons**: Runtime parsing, no compile-time checks

## Recommended Approach

**Option A (Struct-based)** with a thin macro wrapper for ergonomics:

```rust
define_tool!(git, {
    command: "git",
    macos: { brew: "git" },
    linux: { apt: "git", dnf: "git", pacman: "git", apk: "git" },
    windows: { winget: "Git.Git" },
});
```

The macro expands to the full `ToolSpec` struct.

## Implementation Steps

1. Create `src/tools/spec.rs` with `ToolSpec` structs
2. Create `define_tool!` macro in `src/tools/macros.rs`
3. Implement `ToolSpec::ensure()` and `ToolSpec::install()` methods
4. Create central package name registry in `src/tools/packages.rs`
5. Migrate 5 simple tools (git, jq, ripgrep, wget, tree) as proof of concept
6. Add auto-registration via `inventory` or `linkme` crate
7. Migrate remaining 37 tools
8. Remove old boilerplate implementations
9. Update `cargo jarvy new-tool` scaffolding

## Package Name Registry

Create single source of truth for package names:

```rust
// src/tools/packages.rs
pub static PACKAGE_NAMES: phf::Map<(&str, PackageManager), &str> = phf_map! {
    ("docker", Apt) => "docker.io",
    ("docker", Dnf) => "docker",
    ("python", Apt) => "python3",
    ("python", Pacman) => "python",
    ("node", Apt) => "nodejs",
    // ... etc
};
```

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Lines of code in tools/ | ~4,200 | ~1,000 |
| Time to add new tool | 30 min | 5 min |
| Package name locations | 33+ files | 1 file |

## Risks

1. **Complex tools need escape hatch**: nvm, rustup have custom logic
   - Mitigation: Allow `custom_install: fn()` in ToolSpec
2. **Migration breaks existing behavior**
   - Mitigation: Run full test suite after each tool migration
3. **Macro debugging difficulty**
   - Mitigation: Keep macro thin, most logic in regular functions

## Dependencies

- Recommended: `phf` crate for compile-time hash maps
- Optional: `inventory` or `linkme` for auto-registration

## Effort Estimate

- Spec design: 1 day
- Core implementation: 2 days
- Tool migration (42 tools): 3 days
- Testing: 1 day
- Documentation: 0.5 days

## Files to Modify

- `src/tools/mod.rs` - New module structure
- `src/tools/spec.rs` - New file
- `src/tools/macros.rs` - New file
- `src/tools/packages.rs` - New file
- `src/tools/{name}/{name}.rs` - All 42 tool files (migrate or delete)
- `Cargo.toml` - Add phf dependency
