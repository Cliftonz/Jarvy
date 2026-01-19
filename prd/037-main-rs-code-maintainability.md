# PRD-037: Main.rs Code Maintainability Refactor

## Status: Draft
## Priority: P1 (Technical Debt)
## Effort: Medium (3-5 days)

## Overview

Refactor `src/main.rs` (3,085 lines) into well-organized modules following established codebase patterns. The file has grown unwieldy with CLI definitions, command handlers, utility functions, and interactive menus all mixed together, making it difficult to navigate, test, and maintain.

## Problem Statement

1. **Monolithic main.rs**: At 3,085 lines, main.rs is 5-10x larger than recommended module size
2. **Mixed concerns**: CLI definitions, command dispatch, business logic, and utilities are intermingled
3. **Inconsistent patterns**: Some commands delegate to `src/commands/` modules, others have inline handlers
4. **Testing difficulty**: Large inline handlers cannot be unit tested in isolation
5. **Developer friction**: Finding and modifying code requires scrolling through 3000+ lines

## Evidence

- `src/main.rs` is **3,085 lines** vs. typical module sizes of 200-600 lines
- The `main()` function alone spans **1,500+ lines** (lines 615-2140)
- 6 command handler functions are defined inline (team, roles, lock, config, telemetry, user_select)
- Existing `src/commands/` module pattern already handles: doctor, diff, export, upgrade, init, search, validate, completions, templates, quickstart, diagnose

## Current Structure Analysis

| Section | Lines | Description |
|---------|-------|-------------|
| Imports & mod declarations | 1-47 | ~47 lines |
| CLI & Commands enum | 48-401 | ~350 lines of clap definitions |
| Subcommand enums | 403-580 | ~180 lines (Templates, Telemetry, Services, Team, Lock, Config) |
| Helper structs/functions | 582-613 | ~30 lines |
| `main()` function | 615-2140 | **~1,525 lines** - massive match block |
| `handle_telemetry_command` | 2142-2282 | ~140 lines |
| `fetch_remote_config` | 2284-2473 | ~190 lines |
| `user_select`, `print_logo` | 2475-2601 | ~125 lines |
| `handle_team_command` | 2603-2763 | ~160 lines |
| `handle_roles_command` | 2765-2781 | ~17 lines |
| `handle_lock_command` | 2783-2935 | ~150 lines |
| `handle_config_command` | 2937-3085 | ~150 lines |

## Proposed Solution

### New Module Structure

```
src/
├── main.rs              # Entry point only (~100 lines)
├── cli/
│   ├── mod.rs           # Re-exports
│   ├── args.rs          # Cli struct, Commands enum, OutputFormat
│   └── subcommands.rs   # Subcommand enums (TemplatesSubcommand, TelemetryAction, etc.)
├── commands/
│   ├── mod.rs           # Existing + new exports
│   ├── setup.rs         # NEW: Setup command handler (extracted from main)
│   ├── get.rs           # NEW: Get command handler
│   ├── tools_cmd.rs     # NEW: Tools command handler
│   ├── env_cmd.rs       # NEW: Env command handler
│   ├── ci_cmd.rs        # NEW: CiConfig & CiInfo handlers
│   ├── services_cmd.rs  # NEW: Services command handler
│   ├── telemetry_cmd.rs # NEW: Telemetry subcommand handlers
│   ├── team_cmd.rs      # NEW: Team subcommand handlers
│   ├── lock_cmd.rs      # NEW: Lock subcommand handlers
│   ├── config_cmd.rs    # NEW: Config subcommand handlers
│   ├── mcp_cmd.rs       # NEW: MCP command handler
│   └── ... (existing)
├── interactive.rs       # NEW: user_select, print_logo, first-run flow
└── remote.rs            # NEW: fetch_remote_config, transform_github_url
```

### Module Responsibilities

#### 1. `src/cli/args.rs` (~400 lines)
- `Cli` struct with clap derive
- `Commands` enum with all subcommands
- `OutputFormat` enum
- Helper functions like `parse_ci_provider`

#### 2. `src/cli/subcommands.rs` (~150 lines)
- `TemplatesSubcommand`
- `TelemetryAction`
- `ServicesAction`
- `TeamAction`
- `LockAction`
- `ConfigAction`

#### 3. `src/commands/setup.rs` (~500 lines)
- Extract the massive Setup command handler from main()
- Tool installation logic with parallel execution
- Hook execution logic
- Environment variable setup
- Service auto-start

#### 4. `src/commands/get.rs` (~100 lines)
- Get command: collect reports, format output

#### 5. `src/commands/tools_cmd.rs` (~150 lines)
- Tools command: list tools, show index, show default hooks

#### 6. `src/commands/env_cmd.rs` (~150 lines)
- Env command: generate .env, update shell rc

#### 7. `src/commands/services_cmd.rs` (~100 lines)
- Services subcommands: start, stop, status, restart

#### 8. `src/commands/telemetry_cmd.rs` (~150 lines)
- `handle_telemetry_command` (status, enable, disable, set-endpoint, test, preview)
- `update_telemetry_config`

#### 9. `src/commands/team_cmd.rs` (~200 lines)
- `handle_team_command` (add, list, browse, sync, remove, init)

#### 10. `src/commands/lock_cmd.rs` (~200 lines)
- `handle_lock_command` (generate, status, verify)

#### 11. `src/commands/config_cmd.rs` (~200 lines)
- `handle_config_command` (show, refresh)

#### 12. `src/interactive.rs` (~150 lines)
- `user_select()` - interactive menu
- `print_logo()` - ASCII art
- First-run detection and welcome flow

#### 13. `src/remote.rs` (~200 lines)
- `fetch_remote_config()` - URL fetching with caching
- `transform_github_url()` - GitHub URL normalization
- `MAX_REMOTE_CONFIG_SIZE` constant

#### 14. `src/main.rs` (~100 lines)
- Module declarations
- `fn main()` - initialization and command dispatch only
- Command routing to handler modules

## Implementation Steps

### Phase 1: CLI Extraction (Day 1)
1. Create `src/cli/mod.rs`, `args.rs`, `subcommands.rs`
2. Move `Cli`, `Commands`, `OutputFormat` to `args.rs`
3. Move subcommand enums to `subcommands.rs`
4. Update main.rs imports
5. Run `cargo check` and `cargo test`

### Phase 2: Utility Extraction (Day 1)
1. Create `src/remote.rs` with `fetch_remote_config`, `transform_github_url`
2. Create `src/interactive.rs` with `user_select`, `print_logo`
3. Update main.rs imports
4. Run `cargo check` and `cargo test`

### Phase 3: Simple Command Handlers (Day 2)
1. Create `src/commands/get.rs`, `tools_cmd.rs`, `env_cmd.rs`
2. Extract handlers from main() match arms
3. Create `src/commands/services_cmd.rs`
4. Create `src/commands/ci_cmd.rs` for CiConfig and CiInfo
5. Update commands/mod.rs exports
6. Run `cargo check` and `cargo test`

### Phase 4: Complex Command Handlers (Day 3)
1. Move `handle_telemetry_command` to `src/commands/telemetry_cmd.rs`
2. Move `handle_team_command` to `src/commands/team_cmd.rs`
3. Move `handle_lock_command` to `src/commands/lock_cmd.rs`
4. Move `handle_config_command` to `src/commands/config_cmd.rs`
5. Run `cargo check` and `cargo test`

### Phase 5: Setup Command Extraction (Day 4)
1. Create `src/commands/setup.rs`
2. Extract the large Setup handler (~600 lines) from main()
3. Extract helper functions (color_for_status, pretty_output, Reports struct)
4. Run `cargo check` and `cargo test`

### Phase 6: Final Cleanup (Day 5)
1. Clean up main.rs to ~100 lines
2. Run full test suite: `cargo test --verbose`
3. Run clippy: `cargo clippy --all-features -- -D warnings`
4. Run fmt: `cargo fmt --all`
5. Update CLAUDE.md if module patterns changed

## Files to Modify

### New Files
- `src/cli/mod.rs` - CLI module aggregator
- `src/cli/args.rs` - CLI argument definitions
- `src/cli/subcommands.rs` - Subcommand enum definitions
- `src/commands/setup.rs` - Setup command handler
- `src/commands/get.rs` - Get command handler
- `src/commands/tools_cmd.rs` - Tools command handler
- `src/commands/env_cmd.rs` - Env command handler
- `src/commands/ci_cmd.rs` - CI command handlers
- `src/commands/services_cmd.rs` - Services command handler
- `src/commands/telemetry_cmd.rs` - Telemetry command handler
- `src/commands/team_cmd.rs` - Team command handler
- `src/commands/lock_cmd.rs` - Lock command handler
- `src/commands/config_cmd.rs` - Config command handler
- `src/commands/mcp_cmd.rs` - MCP command handler
- `src/interactive.rs` - Interactive menu
- `src/remote.rs` - Remote config fetching

### Modified Files
- `src/main.rs` - Reduce from 3085 to ~100 lines
- `src/commands/mod.rs` - Add new module exports
- `src/lib.rs` - Potentially export cli module for tests

## Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| main.rs lines | 3,085 | ~100 |
| main() function lines | 1,525 | ~80 |
| Largest module | 3,085 (main.rs) | ~500 (setup.rs) |
| Command handlers in main.rs | 6 | 0 |
| All tests pass | Yes | Yes |
| Clippy warnings | 0 | 0 |

## Non-Goals

- Changing any user-facing behavior
- Modifying the CLI interface or command structure
- Refactoring the existing commands/ module implementations
- Performance optimization
- Adding new features

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing tests | Medium | Medium | Run tests after each phase |
| Import path changes break external consumers | Low | Low | lib.rs re-exports unchanged |
| Merge conflicts with parallel work | Medium | Low | Complete in single sprint |
| Missing edge cases in extraction | Low | Medium | Comprehensive test coverage |

## Verification

After implementation:
1. `cargo build` - Compiles without errors
2. `cargo test --verbose -- --show-output` - All tests pass
3. `cargo clippy --all-features -- -D warnings` - No warnings
4. `cargo fmt --all --check` - Properly formatted
5. Manual test: `jarvy --help`, `jarvy setup --dry-run`, `jarvy tools --index`
6. Verify main.rs is under 150 lines
