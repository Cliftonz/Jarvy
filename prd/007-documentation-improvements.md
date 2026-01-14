# PRD-007: Documentation Improvements

## Overview

Fix documentation gaps, inconsistencies, and missing guides to improve developer experience for both users and contributors.

## Problem Statement

Current documentation has several issues:

1. **Typos and broken files**: `Quckstart.md` misspelled and nearly empty
2. **Terminology confusion**: Code uses `[tools]`, docs reference `[provisioner]`
3. **Missing tool catalog**: No list of supported tools
4. **No contributor guide**: "How to add a tool" undocumented
5. **Undocumented features**: Environment variables, version syntax

## Evidence

- `docs/Quckstart.md` has typo in filename and only 9 lines of content
- README shows `[provisioner]` but code uses `[tools]`
- No documentation lists all 40 supported tools
- `cargo jarvy new-tool` mentioned but not explained
- `JARVY_TEST_MODE`, `JARVY_FAST_TEST` used but undocumented

## Requirements

### Functional Requirements

1. **Fix broken docs**: Rename and complete Quickstart
2. **Consistent terminology**: Align docs with code (`[tools]`)
3. **Tool catalog**: List all tools with package manager mappings
4. **Contributor guide**: Step-by-step for adding tools
5. **Config reference**: Complete jarvy.toml documentation
6. **CLI reference**: All commands, flags, env vars

### Non-Functional Requirements

1. Documentation renders correctly on GitHub
2. Examples are copy-paste ready
3. No broken internal links
4. Updated with each release

## Documentation Structure

```
docs/
├── README.md                    # Project overview (keep in root)
├── getting-started/
│   ├── quickstart.md           # 5-minute setup guide
│   ├── installation.md         # Detailed install options
│   └── first-project.md        # Tutorial: add to existing project
├── guides/
│   ├── configuration.md        # Complete jarvy.toml reference
│   ├── version-specification.md # How versions work
│   ├── ci-integration.md       # GitHub Actions, GitLab, etc.
│   └── troubleshooting.md      # FAQ and common issues
├── reference/
│   ├── tools.md                # All supported tools
│   ├── cli.md                  # Command reference
│   ├── environment-variables.md
│   └── error-codes.md          # Already exists, keep updated
├── contributing/
│   ├── adding-a-tool.md        # Step-by-step guide
│   ├── architecture.md         # System design
│   └── testing.md              # How to run/write tests
└── examples/
    ├── node-project.toml
    ├── python-project.toml
    ├── fullstack-project.toml
    └── devops-tools.toml
```

## Document Content

### 1. Quickstart (docs/getting-started/quickstart.md)

```markdown
# Quickstart

Get your development environment set up in under 5 minutes.

## Prerequisites

- macOS, Linux, or Windows
- Package manager installed (Homebrew, apt, winget, etc.)

## Installation

### macOS/Linux
\`\`\`bash
curl -fsSL https://jarvy.dev/install.sh | bash
\`\`\`

### Windows (PowerShell)
\`\`\`powershell
irm https://jarvy.dev/install.ps1 | iex
\`\`\`

## Create Your First Config

Create `jarvy.toml` in your project root:

\`\`\`toml
[tools]
git = "latest"
node = "20"
python = "3.12"
docker = "latest"
\`\`\`

## Run Setup

\`\`\`bash
jarvy setup
\`\`\`

## Verify Installation

\`\`\`bash
jarvy get
\`\`\`

Output:
\`\`\`
Tool     | Required | Installed | Status
---------|----------|-----------|--------
git      | latest   | 2.44.0    | ✓ Match
node     | 20       | 20.10.0   | ✓ Match
python   | 3.12     | 3.12.1    | ✓ Match
docker   | latest   | 24.0.7    | ✓ Match
\`\`\`

## Next Steps

- [Configuration Guide](../guides/configuration.md)
- [Supported Tools](../reference/tools.md)
- [CI Integration](../guides/ci-integration.md)
```

### 2. Tools Reference (docs/reference/tools.md)

```markdown
# Supported Tools

Jarvy supports 40+ tools across multiple categories.

## Languages & Runtimes

| Tool | Command | macOS | Linux | Windows | Notes |
|------|---------|-------|-------|---------|-------|
| node | `node` | Homebrew | apt/dnf | winget | Includes npm |
| python | `python3` | Homebrew | apt/dnf | winget | Python 3.x |
| go | `go` | Homebrew | apt/dnf | winget | |
| rust | `rustc` | rustup | rustup | rustup | Via rustup |
| ruby | `ruby` | Homebrew | apt/dnf | winget | |

## Infrastructure & DevOps

| Tool | Command | macOS | Linux | Windows | Notes |
|------|---------|-------|-------|---------|-------|
| docker | `docker` | Cask | apt/dnf | winget | Docker Desktop |
| terraform | `terraform` | Homebrew | apt | winget | |
| kubectl | `kubectl` | Homebrew | apt | winget | |

## Utilities

| Tool | Command | macOS | Linux | Windows | Notes |
|------|---------|-------|-------|---------|-------|
| git | `git` | Homebrew | apt/dnf | winget | |
| jq | `jq` | Homebrew | apt/dnf | winget | JSON processor |
| ripgrep | `rg` | Homebrew | apt/dnf | winget | Fast grep |

## Package Names by Manager

<details>
<summary>Click to expand package name mappings</summary>

| Tool | Homebrew | apt | dnf | pacman | winget |
|------|----------|-----|-----|--------|--------|
| docker | docker | docker.io | docker | docker | Docker.DockerDesktop |
| python | python | python3 | python3 | python | Python.Python.3 |
| node | node | nodejs | nodejs | nodejs | OpenJS.NodeJS.LTS |

</details>

## Requesting New Tools

Open an issue with the "tool request" template.
```

### 3. Configuration Reference (docs/guides/configuration.md)

```markdown
# Configuration Reference

Complete reference for `jarvy.toml`.

## File Location

Jarvy looks for `jarvy.toml` in the current directory.

## Basic Syntax

\`\`\`toml
[tools]
# Simple format: tool = "version"
git = "latest"
node = "20"

# Detailed format
[tools.python]
version = "3.12"
use_sudo = false
\`\`\`

## Tools Section

### Simple Format
\`\`\`toml
[tools]
git = "2.40"      # Prefix match: accepts 2.40.x
node = "20.10.0"  # Exact version
python = "latest" # Skip version check
docker = "*"      # Any version
\`\`\`

### Detailed Format
\`\`\`toml
[tools.node]
version = "20"
version_manager = true  # Use nvm instead of system package
use_sudo = false        # Override global sudo setting
\`\`\`

## Version Specification

| Syntax | Meaning | Example Match |
|--------|---------|---------------|
| `"3.10"` | Prefix | 3.10.0, 3.10.5 |
| `"3.10.0"` | Exact | 3.10.0 only |
| `">= 3.10"` | Minimum | 3.10, 3.11, 4.0 |
| `"< 4.0"` | Maximum | 3.x |
| `">= 3.10, < 4.0"` | Range | 3.10 - 3.99 |
| `"latest"` | Any | Skip check |
| `"*"` | Any | Skip check |

## Privileges Section

\`\`\`toml
[privileges]
use_sudo = true  # Default: auto-detect
\`\`\`

## Complete Example

\`\`\`toml
# jarvy.toml for a full-stack Node.js project

[tools]
git = "latest"
node = "20"
python = "3.12"
docker = "latest"
terraform = "1.6"
kubectl = "latest"
jq = "latest"
ripgrep = "latest"

[tools.node]
version = "20"
version_manager = true  # Use nvm

[privileges]
use_sudo = false  # Running as non-root in container
\`\`\`
```

### 4. Adding a Tool Guide (docs/contributing/adding-a-tool.md)

```markdown
# Adding a New Tool

Step-by-step guide for contributing a new tool to Jarvy.

## Prerequisites

- Rust toolchain installed
- Fork of the Jarvy repository
- Basic familiarity with Rust

## Step 1: Scaffold the Tool

\`\`\`bash
cargo run -p cargo-jarvy -- new-tool mytool
\`\`\`

This creates:
- `src/tools/mytool/mod.rs`
- `src/tools/mytool/mytool.rs`

## Step 2: Define Package Names

Edit `src/tools/mytool/mytool.rs`:

\`\`\`rust
fn install_macos() -> Result<(), InstallError> {
    // Homebrew formula name
    PkgOps::install(PackageManager::Homebrew, "mytool-formula", None)
}

fn install_linux() -> Result<(), InstallError> {
    let pm = detect_linux_pm().ok_or(InstallError::Prereq("No package manager"))?;

    // Package names vary by distro
    let pkg = match pm {
        PackageManager::Apt => "mytool",
        PackageManager::Dnf => "mytool",
        PackageManager::Pacman => "mytool-bin",
        PackageManager::Apk => "mytool",
        _ => "mytool",
    };

    PkgOps::install(pm, pkg, None)
}

fn install_windows() -> Result<(), InstallError> {
    // Winget ID (find at https://winget.run)
    PkgOps::install(PackageManager::Winget, "Publisher.MyTool", None)
}
\`\`\`

## Step 3: Register the Tool

Add to `src/tools/mod.rs` in `register_all()`:

\`\`\`rust
let _ = register_tool("mytool", crate::tools::mytool::mytool::add_handler);
\`\`\`

## Step 4: Test Locally

\`\`\`bash
# Build
cargo build

# Test registration
cargo test --test tools_matrix

# Test installation (careful: actually installs!)
./target/debug/jarvy setup  # with jarvy.toml containing mytool
\`\`\`

## Step 5: Add Tests

In `src/tools/mytool/mytool.rs`:

\`\`\`rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_mytool_no_panic() {
        std::env::set_var("JARVY_FAST_TEST", "1");
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
\`\`\`

## Step 6: Submit PR

1. Run `cargo fmt && cargo clippy`
2. Ensure all tests pass: `cargo test`
3. Update docs/reference/tools.md with your tool
4. Create PR with conventional commit: `feat(tools): add mytool support`

## Common Issues

### Finding Package Names

- **Homebrew**: `brew search mytool`
- **apt**: `apt search mytool`
- **dnf**: `dnf search mytool`
- **winget**: https://winget.run or `winget search mytool`
- **pacman**: https://archlinux.org/packages/

### Version Detection

Most tools use `--version`, but some use `-V` or `version`:

\`\`\`rust
pub fn ensure(min_hint: &str) -> Result<(), InstallError> {
    // Custom version flag
    if let Ok(out) = Command::new("mytool").arg("-V").output() {
        let version = String::from_utf8_lossy(&out.stdout);
        if version.contains(min_hint) {
            return Ok(());
        }
    }
    install()
}
\`\`\`
```

### 5. Environment Variables (docs/reference/environment-variables.md)

```markdown
# Environment Variables

Jarvy behavior can be configured via environment variables.

## Runtime Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `JARVY_CONFIG` | Path to jarvy.toml | `./jarvy.toml` |
| `JARVY_HOME` | Jarvy data directory | `~/.jarvy` |
| `JARVY_VERBOSE` | Enable verbose output | unset |
| `JARVY_NO_COLOR` | Disable colored output | unset |

## Telemetry Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `JARVY_OTLP_ENDPOINT` | OpenTelemetry endpoint | unset |
| `JARVY_DISABLE_TELEMETRY` | Disable all telemetry | unset |
| `POSTHOG_API_KEY` | PostHog analytics key | built-in |

## Test Variables

Used for development and CI:

| Variable | Description |
|----------|-------------|
| `JARVY_TEST_MODE` | Disables interactive prompts |
| `JARVY_FAST_TEST` | Skips external command execution |
| `JARVY_MOCK_COMMANDS` | Enables command mocking in tests |

## Example Usage

\`\`\`bash
# Verbose setup
JARVY_VERBOSE=1 jarvy setup

# Custom config location
JARVY_CONFIG=/path/to/jarvy.toml jarvy setup

# Disable telemetry
JARVY_DISABLE_TELEMETRY=1 jarvy setup
\`\`\`
```

## Implementation Steps

1. Rename `docs/Quckstart.md` → `docs/getting-started/quickstart.md`
2. Create new directory structure
3. Write quickstart guide with working examples
4. Create tools reference with all 40 tools
5. Write complete configuration reference
6. Create "adding a tool" contributor guide
7. Document all environment variables
8. Add example jarvy.toml files
9. Fix terminology: replace `[provisioner]` with `[tools]`
10. Add troubleshooting FAQ
11. Update README to link to new docs

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Documentation pages | 5 | 15+ |
| Example configs | 0 | 5+ |
| Tool catalog | None | Complete |
| Contributor guide | None | Complete |

## Risks

1. **Docs drift from code**: Documentation becomes outdated
   - Mitigation: Add CI check for doc/code alignment
   - Mitigation: Include docs updates in PR template
2. **Over-documentation**: Too verbose, hard to navigate
   - Mitigation: Keep guides focused, use expandable sections

## Dependencies

None - pure documentation work.

## Effort Estimate

- Fix Quickstart: 0.5 days
- Tools reference: 1 day
- Configuration guide: 0.5 days
- Contributor guide: 0.5 days
- Environment variables: 0.25 days
- Examples: 0.25 days
- Review and polish: 0.5 days

## Files to Create/Modify

- `docs/Quckstart.md` → Delete
- `docs/getting-started/quickstart.md` - New
- `docs/getting-started/installation.md` - New
- `docs/guides/configuration.md` - New
- `docs/reference/tools.md` - New
- `docs/reference/environment-variables.md` - New
- `docs/contributing/adding-a-tool.md` - New
- `docs/examples/*.toml` - New example files
- `README.md` - Update links, fix terminology
