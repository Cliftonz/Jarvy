# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                                    # Debug build
cargo build --release                          # Release build
cargo fmt --all                                # Format code
cargo clippy --all-features -- -D warnings     # Lint (must pass for CI)
cargo check --verbose                          # Type check
cargo test --verbose -- --show-output          # Run all tests
cargo test --test cli_dispatch -- --show-output  # Run single integration test
cargo run -p cargo-jarvy -- new-tool <name>    # Scaffold a new tool
```

## Architecture

Jarvy is a cross-platform CLI tool that provisions development environments from a `jarvy.toml` config file. It uses native package managers (Homebrew on macOS, apt/dnf/etc on Linux, winget/Chocolatey on Windows).

### Core Modules

- **`src/main.rs`** - CLI entry point using clap with derive macros. Commands: `setup`, `bootstrap`, `configure`, `get`, `roles`
- **`src/config.rs`** - Parses `jarvy.toml` using serde. Supports simple (`git = "2.40"`) and detailed (`git = { version = "2.40", version_manager = true }`) formats
- **`src/roles/`** - Role-based configurations with inheritance (PRD-033). Key files: `definition.rs` (types), `resolver.rs` (inheritance), `commands.rs` (CLI)
- **`src/tools/registry.rs`** - Global `OnceLock<RwLock<HashMap>>` registry mapping tool names to handler functions
- **`src/tools/common.rs`** - Shared utilities: `Os` enum, `InstallError` type, `run()`, `has()`, `cmd_satisfies()`, package manager detection
- **`src/tools/spec.rs`** - ToolSpec pattern: `ToolSpec` struct and `define_tool!` macro for declarative tool definitions

### Tool Implementation Pattern

Tools use the declarative `define_tool!` macro for minimal boilerplate:

```rust
//! jq - JSON processor
use crate::define_tool;

define_tool!(JQ, {
    command: "jq",
    macos: { brew: "jq" },
    linux: { uniform: "jq" },
    windows: { winget: "jqlang.jq" },
});
```

Each tool lives in `src/tools/{name}/` with two files:
- `mod.rs` - Re-exports with `pub use {name}::*;`
- `{name}.rs` - Tool definition using `define_tool!` macro

**Macro variants:**
- `macos: { brew: "pkg" }` - Homebrew formula
- `macos: { cask: "pkg" }` - Homebrew cask (GUI apps)
- `linux: { uniform: "pkg" }` - Same package name across all distros
- `linux: { apt: "x", dnf: "y", pacman: "z", apk: "w" }` - Different names per package manager
- `windows: { winget: "Publisher.Package" }` - Winget package ID
- `custom_install: Some(fn_name)` - For tools needing shell scripts (nvm, rustup, brew)
- `default_hook: { description: "...", script: "..." }` - Post-install hook that runs automatically

Tools are registered in `src/tools/mod.rs` via `register_all()`.

### Default Hooks

Tools can define built-in post-install hooks that configure the tool after installation. Default hooks are:
- **Idempotent** - Safe to run multiple times (scripts check before modifying files)
- **Advisory** - Failures are warnings, not errors; setup continues
- **Overridable** - User-defined `[hooks.tool]` takes precedence

Example tool with default hook:
```rust
define_tool!(STARSHIP, {
    command: "starship",
    macos: { brew: "starship" },
    linux: { uniform: "starship" },
    windows: { winget: "Starship.Starship" },
    default_hook: {
        description: "Add starship shell initialization to .bashrc and .zshrc",
        script: r#"
# Add to .zshrc if not present
if [ -f "$HOME/.zshrc" ] && ! grep -q 'starship init zsh' "$HOME/.zshrc"; then
    echo 'eval "$(starship init zsh)"' >> "$HOME/.zshrc"
fi
"#
    },
});
```

List tools with default hooks: `jarvy tools --default-hooks`

### Tool Dependencies

Tools can declare dependencies on other tools to ensure correct installation order:

**Strict Dependencies** (`depends_on`): ALL listed tools MUST be available.
```rust
define_tool!(LAZYDOCKER, {
    command: "lazydocker",
    macos: { brew: "lazydocker" },
    // Docker TUI requires Docker daemon
    depends_on: &["docker"],
});
```

**Flexible Dependencies** (`depends_on_one_of`): AT LEAST ONE tool must be available.
```rust
define_tool!(KUBECTL, {
    command: "kubectl",
    macos: { brew: "kubectl" },
    // Needs any K8s cluster provider (minikube, kind, docker, etc.)
    depends_on_one_of: &["minikube", "kind", "k3d", "docker", "podman"],
});
```

**Dependency Behavior:**
- Strict: Missing deps cause warnings; tools are still installed but may not work
- Flexible: If one option is installed, satisfied. If one is in config, it's installed first. Otherwise, advisory warning
- Dependencies affect installation order via topological sort

**Dependency Functions** (`src/tools/spec.rs`):
- `get_tool_dependencies()` - Get strict dependencies
- `get_tool_flexible_dependencies()` - Get flexible dependencies
- `check_tool_dependencies()` - Returns `DependencyCheckResult` (Satisfied, MissingRequired, WillInstallFlexible, MissingFlexible)
- `order_tools_by_dependencies()` - Topological sort respecting both dependency types

### Role-Based Configurations

Jarvy supports role-based tool configurations for teams with diverse developer roles. Each role defines a tool set that gets merged with directly configured tools.

**Config Example:**
```toml
# Assign a role
role = "frontend"
# Or multiple roles (last wins for conflicts)
role = ["frontend", "devops"]

# Direct tools always override role tools
[provisioner]
vim = "latest"

# Role definitions
[roles.base]
description = "Base development tools"
tools = ["git", "docker"]

[roles.frontend]
description = "Frontend development"
extends = "base"                    # Inherit from parent
tools = ["node", "bun"]

[roles.frontend.tools]              # Version overrides
node = "20"
bun = "latest"

[roles.senior-frontend]
extends = "frontend"                # Deep inheritance (max 5 levels)
tools = ["kubectl"]
```

**CLI Commands:**
```bash
jarvy roles list                    # List available roles
jarvy roles list -v                 # Verbose with tool counts
jarvy roles show <name>             # Show role details
jarvy roles show <name> --resolved  # Show with inherited tools
jarvy roles show <name> --inheritance  # Show inheritance chain
jarvy roles diff <a> <b>            # Compare two roles
jarvy setup --role <name>           # Override role for single run
```

**Module**: `src/roles/` - Role definitions, resolution with inheritance, and CLI commands.

### Config Files

- **`jarvy.toml`** (project) - Tools to provision with versions
- **`~/.jarvy/config.toml`** (global) - Telemetry settings, machine fingerprint

### Telemetry

Jarvy uses OpenTelemetry (OTEL) for unified telemetry: logs, metrics, and optional traces. Telemetry is **opt-in by default** (disabled until configured).

**Configuration** (`~/.jarvy/config.toml`):
```toml
[telemetry]
enabled = true                          # Master switch
endpoint = "http://localhost:4318"      # OTLP endpoint
protocol = "http"                       # "http" or "grpc"
logs = true                             # Log export
metrics = true                          # Metrics export
traces = false                          # Optional traces
sample_rate = 1.0                       # Trace sampling (0.0-1.0)
```

**Environment Variables** (override config file):
- `JARVY_TELEMETRY=1` - Enable/disable telemetry
- `JARVY_OTLP_ENDPOINT=http://...` - OTLP endpoint URL
- `JARVY_OTLP_PROTOCOL=grpc` - Protocol selection
- `JARVY_OTLP_LOGS=1`, `JARVY_OTLP_METRICS=1`, `JARVY_OTLP_TRACES=1`
- `JARVY_OTLP_SAMPLE_RATE=0.1` - Trace sampling rate

**CI Behavior**: Auto-disabled when `CI=true` unless `JARVY_TELEMETRY=1` is set.

**CLI Commands**:
```bash
jarvy telemetry status        # Show current config
jarvy telemetry enable        # Enable telemetry
jarvy telemetry disable       # Disable telemetry
jarvy telemetry set-endpoint <url>  # Set OTLP endpoint
jarvy telemetry test          # Test connectivity
jarvy telemetry preview       # Show what events would be sent
```

**Module**: `src/telemetry.rs` - Unified telemetry API with event functions and metrics.

## Testing

Integration tests are in `/tests/`. Key test env vars:
- `JARVY_TEST_MODE=1` - Disables interactive prompts
- `JARVY_FAST_TEST` - Skips external command execution

## Exit Codes

- `0` - Success
- `2` - CONFIG_ERROR (malformed jarvy.toml)
- `3` - PREREQ_MISSING (package manager not found)
- `5` - PERMISSION_REQUIRED (sudo needed)

## Conventions

- Rust 2024 edition idioms
- Conventional Commits: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`
- Prefer stdlib and existing dependencies over new crates
- Run `cargo fmt` and `cargo clippy` before committing
