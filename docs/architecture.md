---
title: "Architecture - Jarvy"
description: "How Jarvy is organized: module map, control flow, key patterns. Reference for contributors and AI agents."
---

# Architecture

This page is the source-of-truth module map. It is written for two audiences:

- **Contributors** wiring a new feature and asking "where does this belong?"
- **AI agents** reading the codebase to make targeted edits without re-discovering structure

## Bird's-Eye View

```
jarvy.toml ──► Config parser (src/config.rs)
                    │
                    ▼
            Resolver (roles, extends, defaults)
                    │
                    ▼
            Command dispatch (src/main.rs ── src/commands/*)
                    │
        ┌───────────┼───────────────┐
        ▼           ▼               ▼
   Provisioner   Hooks         Sub-systems
   (parallel     (pre/post     (env, git, packages,
   tool install) per-tool)      services, drift, etc.)
        │
        ▼
   Tool registry (src/tools/registry.rs)
   ── declarative `define_tool!` definitions
        │
        ▼
   Native package managers
   (brew / apt / dnf / pacman / apk / winget / choco / scoop)
```

Everything else — telemetry, logging, MCP server, self-update — wraps or observes this core flow.

## Module Map

### Top-Level Entry Points

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry. Parses args, sets up telemetry, dispatches commands. ~540 lines. Keep it thin. |
| `src/lib.rs` | Library entry — re-exports public API for `cargo-jarvy` and tests |
| `build.rs` | Generates the JSON tool-index used by `jarvy schema` and the MCP server |

### CLI Layer

| Module | Purpose |
|--------|---------|
| `src/cli/args.rs` | `Cli` struct, `Commands` enum, `OutputFormat`. Top-level `clap` definitions. |
| `src/cli/subcommands.rs` | Nested enums: `TemplatesSubcommand`, `TelemetryAction`, `ServicesAction`, `TeamAction`, `LockAction`, `ConfigAction`, `UpdateSubcommand`. |
| `src/commands/*` | One file per top-level command. Each exports a `run(args) -> Result<()>` style entry. |

### Configuration

| Module | Purpose |
|--------|---------|
| `src/config.rs` | TOML deserialization, field defaults, validation. The schema. |
| `src/remote.rs` | Fetch + cache remote configs (`extends = "https://..."`), handle GitHub URL transforms. |
| `src/roles/*` | Role-based tool sets with inheritance. `definition.rs` (types), `resolver.rs` (inheritance), `commands.rs` (CLI). |

### Tool System

| Module | Purpose |
|--------|---------|
| `src/tools/spec.rs` | `ToolSpec` struct and `define_tool!` macro. The declarative format every tool uses. |
| `src/tools/registry.rs` | Global `OnceLock<RwLock<HashMap>>` mapping tool name → handler. Populated at startup via `register_all()`. |
| `src/tools/common.rs` | Shared helpers: `Os` enum, `InstallError`, `run()`, `has()`, `cmd_satisfies()`, package manager detection. |
| `src/tools/<name>/<name>.rs` | One tool per directory. Each calls `define_tool!`. |
| `src/tools/mod.rs` | Re-exports + `register_all()` |

### Subsystems

Each lives in its own module and follows a similar shape: `config.rs` (types), supporting files, optional `commands.rs` for CLI surface.

| Module | Feature | Doc |
|--------|---------|-----|
| `src/drift/` | Drift detection, baseline state | [drift.md](drift.md) |
| `src/update/` | Self-updating, channels, rollback | [self-update.md](self-update.md) |
| `src/logging/` | File logging, rotation, sanitization | [logging.md](logging.md) |
| `src/ticket/` | Debug ticket bundles | [logging.md](logging.md) |
| `src/network/` | Proxy, TLS, env propagation | [network.md](network.md) |
| `src/git/` | Git config automation | [git-config.md](git-config.md) |
| `src/packages/` | npm/pip/cargo packages | [packages.md](packages.md) |
| `src/services/` | Docker Compose + Tilt orchestration | [configuration.md](configuration.md#services-services) |
| `src/env/` | Env vars, secrets, dotenv generation | [configuration.md](configuration.md#environment-variables-env) |
| `src/team/` | Shared team config sources | [configuration.md](configuration.md) |
| `src/lock/` | Tool version locking | – |
| `src/mcp/` | MCP server for AI agents | [mcp-server.md](mcp-server.md) |
| `src/observability/`, `src/telemetry.rs` | OTEL signals | [telemetry.md](telemetry.md) |
| `src/templates/` | Project init templates | [quickstart.md](quickstart.md) |
| `src/onboarding/`, `src/init.rs` | First-run experience | – |
| `src/ci/` | CI provider detection + config generation | [ci-cd.md](ci-cd.md) |
| `src/output/`, `src/outputs.rs` | JSON/text output formatters | – |
| `src/error_codes.rs` | Exit-code constants | [error-codes.md](error-codes.md) |

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `crates/cargo-jarvy/` | Cargo subcommand for scaffolding new tools (`cargo jarvy new-tool <name>`) |

## Control Flow: `jarvy setup`

The most important code path. Read this once and you understand 70% of Jarvy.

1. **Parse args** (`src/cli/args.rs`)
2. **Init telemetry** (`src/telemetry.rs`) — OTEL pipeline, log writer
3. **Load config** (`src/config.rs`)
   - Resolve `extends` recursively (`src/remote.rs`)
   - Apply role inheritance (`src/roles/resolver.rs`)
   - Merge env-var overrides
4. **Build install plan** (`src/commands/setup_cmd.rs`)
   - Filter to relevant role
   - Topologically sort by dependencies (`src/tools/spec.rs::order_tools_by_dependencies`)
5. **Pre-setup hook** (`src/hooks.rs`)
6. **Install tools in parallel** (`rayon` thread pool, default 4 jobs)
   - Each tool: detect → check satisfied → install via package manager → verify
   - Per-tool `post_install` hook fires immediately after success
7. **Default hooks** for tools that ship with one (e.g. starship shell init)
8. **Language packages** (`src/packages/`) — npm, pip, cargo
9. **Git config** (`src/git/`)
10. **Env vars + dotenv** (`src/env/`)
11. **Services start** (`src/services/`) — Compose / Tilt
12. **Post-setup hook**
13. **Drift baseline write** (`src/drift/state.rs`) — only on full success

If anything fails: bail with the matching exit code from `src/error_codes.rs`. Per-tool failures don't stop the parallel batch unless `continue_on_error = false`.

## Key Patterns

### Declarative Tools via `define_tool!`

Adding a tool is one file:

```rust
// src/tools/jq/jq.rs
use crate::define_tool;

define_tool!(JQ, {
    command: "jq",
    macos: { brew: "jq" },
    linux: { uniform: "jq" },
    windows: { winget: "jqlang.jq" },
});
```

`mod.rs` does `pub use jq::*;`. `src/tools/mod.rs` calls `JQ::register()` in `register_all()`. Done.

The macro generates: install fn, version-check fn, registry handle, optional default-hook bindings.

### Global Registry via `OnceLock<RwLock<HashMap>>`

Tools are registered once at startup (`register_all()`). The registry is read-only for the rest of the process lifetime, so `RwLock` reads are uncontended. No globals problem because the only mutation is during initialization.

### Config Validation at Boundary

Parse → validate → use. `src/config.rs` deserializes raw TOML into the same structs the rest of the code uses. By the time other modules touch config, every value has already been checked. Internal code does not re-validate.

### Layered Sub-Systems

Each subsystem (drift, telemetry, etc.) is independent: it has its own `config.rs`, internal types, and CLI surface. Cross-cutting concerns (errors, exit codes) are shared via `src/error_codes.rs` and the common error type. Adding a new subsystem is a directory + a wire-up call from `main.rs` or `setup_cmd.rs`.

### Native Package Managers, Not Wrappers

Jarvy never bundles its own tool binaries. It runs `brew install`, `apt-get install`, `winget install`. Reasons:

- Users get the trust model of their package manager (signatures, mirrors, security updates)
- Versions and security advisories are handled upstream
- No binary distribution responsibility on Jarvy maintainers

The cost: Jarvy can only install what the host's package managers know about. The `define_tool!` macro hides this asymmetry per-tool.

## Testing

| Layer | Where | How |
|-------|-------|-----|
| Unit | inside each module, `#[cfg(test)] mod tests` | `cargo test` |
| Integration | `/tests/*.rs` | `cargo test --test <name>` |
| End-to-end | `tests/e2e/*` and EC2 harness | `JARVY_FAST_TEST=0 cargo test` |
| Benches | `benches/*` | `cargo bench` |

Critical env flags:
- `JARVY_TEST_MODE=1` — disable interactive prompts
- `JARVY_FAST_TEST=1` — skip external command execution (use mocks)

See [`docs/e2e-testing-harness.md`](e2e-testing-harness.md) for the EC2 harness.

## Build

| Profile | Use |
|---------|-----|
| `dev` | `opt-level = 1` for faster local iteration |
| `release` | `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true` |

Workspace lints (deny clippy::correctness, warn suspicious/perf/style/complexity, warn unsafe_code) apply to every crate.

## Where to Make Common Changes

| Goal | Edit |
|------|------|
| Add a tool | `src/tools/<name>/{mod.rs,<name>.rs}` + register in `src/tools/mod.rs` |
| Add a CLI command | `src/cli/args.rs` (variant) + `src/cli/subcommands.rs` (nested enum) + `src/commands/<name>.rs` |
| Add a config field | `src/config.rs` struct + default + validation |
| Add a hook variant | `src/hooks.rs` |
| Add a CI provider | `src/ci/detection.rs` + `src/ci/generators/<name>.rs` |
| Add an MCP tool | `src/mcp/tools.rs` and update the JSON-RPC handler |
| Add a telemetry metric | `src/telemetry.rs` — define under `Metrics::new()` |

## See Also

- [CLI Reference](cli.md)
- [Configuration Reference](configuration.md)
- [Adding Tools](adding-tools.md)
- [Contributing](contributing.md)
- [For AI Agents](for-ai-agents.md)
