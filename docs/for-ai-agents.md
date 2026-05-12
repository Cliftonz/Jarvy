---
title: "For AI Agents - Jarvy"
description: "Everything an AI assistant needs to use, integrate with, or modify Jarvy. MCP server, llms.txt, architecture map, common tasks."
---

# For AI Agents

This page is written for AI assistants (Claude, GPT, Gemini, Cursor, Copilot, internal agents) that interact with Jarvy in any of three modes:

1. **Use Jarvy** — install or check tools on the user's behalf
2. **Configure Jarvy** — generate or edit `jarvy.toml` for a project
3. **Modify Jarvy** — contribute code to the Jarvy repo

Each mode has different needs. Jump to the section that matches your task.

If the user said **"integrate Jarvy into this project"** or **"implement
https://jarvy.dev into this project"** or any close variant — read the
[Integration Quickstart](#integration-quickstart-integrate-jarvy-into-this-project)
below first. That's the canonical first-contact workflow.

---

## Integration Quickstart: "Integrate Jarvy Into This Project"

This is the canonical workflow when a user asks an AI assistant in their
own repo to add Jarvy to the project. The end state is a committed
`jarvy.toml` (plus optionally a `Makefile` + `scripts/bootstrap.sh`) that
any teammate or new laptop can use to bring up a working dev environment
in one command.

### Step 1: Confirm Jarvy is installed on the user's machine

```bash
jarvy --version
```

If the command isn't found, ask the user to install Jarvy first. **Do not
auto-install** — Jarvy runs with elevated privileges and the user should
opt in explicitly. Point them at:

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/bearbinary/jarvy/main/dist/scripts/install.sh | bash

# Or
cargo install jarvy
```

```powershell
# Windows
irm https://raw.githubusercontent.com/bearbinary/jarvy/main/dist/scripts/install.ps1 | iex
```

Wait for the user to confirm install before continuing.

### Step 2: Detect the project's stack

Look for stack-defining files in the repo root and immediate subdirectories.
Use them to pick the right template (full table:
[Templates Decision Table](templates-index.md#decision-table)):

| Signal | Template |
|--------|----------|
| `package-lock.json` only | `node-npm` |
| `pnpm-lock.yaml` | `node-pnpm` |
| `bun.lockb` or `bunfig.toml` | `node-bun` |
| `deno.json` or `deno.jsonc` | `deno` |
| `uv.lock`, or `pyproject.toml` with `[tool.uv]` | `python-uv` |
| `requirements.txt`, no uv | `python-api` |
| `go.mod` (HTTP service shape) | `go-api` |
| `Cargo.toml` with `[workspace]` | `rust-workspace` |
| `Cargo.toml` single `[package]` | `rust-cli` |
| `Gemfile` + Rails layout | `ruby-rails` |
| `pom.xml` with Spring Boot parent | `java-spring` |
| React/Vite frontend | `react-app` |
| Mixed frontend + backend monorepo | `fullstack` |
| Terraform/Helm/K8s tooling repo | `k8s-platform` |

If two templates match equally, ask the user which one fits. If nothing
matches, start from `examples/node-npm` or `examples/python-api` as the
closest analog and tell the user to expect more customization.

### Step 3: Fetch the template into the project

```bash
TEMPLATE=<picked-template>
curl -fsSL \
  "https://raw.githubusercontent.com/bearbinary/jarvy/main/examples/${TEMPLATE}/jarvy.toml" \
  -o jarvy.toml
```

If the user wants the clean-laptop bootstrap as well (gives `make setup`
as the one-command path for new contributors), also fetch:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/bearbinary/jarvy/main/Makefile \
  -o Makefile
mkdir -p scripts
curl -fsSL \
  https://raw.githubusercontent.com/bearbinary/jarvy/main/scripts/bootstrap.sh \
  -o scripts/bootstrap.sh
chmod +x scripts/bootstrap.sh
```

### Step 4: Customize `jarvy.toml` to the project's actual versions

Read the project's existing version hints and reflect them in `jarvy.toml`:

| Source | Maps to |
|--------|---------|
| `.nvmrc` or `package.json` `engines.node` | `node = "<version>"` |
| `.python-version` or `pyproject.toml` `requires-python` | `python = "<version>"` |
| `go.mod` `go <version>` | `go = "<version>"` |
| `rust-toolchain.toml` `channel` | `rust = "<channel>"` |
| `.tool-versions` (asdf/mise) | one line per tool |
| `.ruby-version` | `ruby = "<version>"` |

**Do not** invent versions. If a hint file is missing, leave the template's
default and ask the user to confirm before commit.

Strip sections the project doesn't need (e.g. drop `[git]` if the team
already manages git config; drop `[network]` if no corporate proxy).

### Step 5: Validate

```bash
jarvy validate          # Schema + value check
jarvy diff              # What would change on this machine
jarvy setup --dry-run   # Full execution plan, no mutations
```

If `jarvy validate` fails, fix the reported issues and re-run. **Never run
`jarvy setup` for real until `validate` and `--dry-run` both look right.**

### Step 6: Get explicit user confirmation before any non-dry-run

Surface the dry-run output to the user. Ask whether to:

1. Just commit the config so teammates can run it themselves, OR
2. Also run `jarvy setup` now to provision this machine.

Wait for the user to pick. Do not pick for them.

### Step 7: Commit the config

```bash
git add jarvy.toml
# If Makefile + bootstrap.sh were also fetched:
git add Makefile scripts/bootstrap.sh
git commit -m "feat(devenv): provision via Jarvy"
```

For team rollout, update the project's contributing or onboarding doc to
mention `make setup` (if `Makefile` was added) or `jarvy setup` (if not).

### Step 8: Optional — wire CI

If the project has CI, also offer to generate a config that installs the
same tools in CI so dev/CI parity is enforced:

```bash
jarvy ci-config github         # Or: gitlab | circleci | etc.
```

This emits a CI snippet the user can paste into their existing workflow,
or a complete workflow file if they don't have one yet.

### Anti-patterns during integration

- **Don't run `jarvy setup` without `--dry-run` first.** Jarvy installs
  with elevated privileges (sudo on Linux, Homebrew/winget elsewhere).
  Always show the user what will happen before it happens.
- **Don't auto-commit secrets.** If the template references a secret, use
  `{ env = "VAR_NAME" }` indirection so the secret stays in the user's
  shell or 1Password, not in version control.
- **Don't pin every tool to `latest`.** Pin at least the major version
  (`node = "20"`) so version drift is bounded across the team.
- **Don't duplicate tools the registry already has.** Run `jarvy search
  <tool>` first — there are 174+ tools already defined. Only fall back
  to a custom hook if `jarvy search` returns nothing.
- **Don't bypass roles.** If two teammates need different tool sets, model
  it with `[roles.X]`, not by maintaining separate `jarvy.toml` files.

### Reference

- [Templates index](templates-index.md) — full decision table + per-template details
- [Configuration reference](configuration.md) — every field, every section
- [Roles](roles.md) — for projects with multiple developer roles
- [CI/CD](ci-cd.md) — for the optional Step 8

---

## Mode 1: Use Jarvy on Behalf of the User

### Preferred channel: MCP server

Jarvy ships an [MCP server](mcp-server.md) that gives you typed, rate-limited, audited access to tool installs. Always prefer MCP over shell-invoking the CLI directly — it has built-in safety (dry-run by default, allowlists, audit log).

**Quick start (Claude Desktop):**

```json
{
  "mcpServers": {
    "jarvy": {
      "command": "jarvy",
      "args": ["mcp"]
    }
  }
}
```

**Available MCP tools:**

| Tool | Use |
|------|-----|
| `jarvy_list_tools` | Discover what's installable |
| `jarvy_get_tool` | Get install methods + dependencies |
| `jarvy_check_tool` | "Is X installed?" |
| `jarvy_check_multiple` | Bulk version check |
| `jarvy_install_tool` | Install (dry-run by default) |

**Available MCP resources:**

- `jarvy://tools/index` — full tool catalog as JSON
- `jarvy://platform/info` — host OS, arch, package managers
- `jarvy://tools/{name}` — per-tool detail

**Available MCP prompts:**

- `setup_dev_environment` — guided env setup, accepts `project_type`
- `diagnose_missing_tools` — checks common dev tools

Full reference: [mcp-server.md](mcp-server.md).

### Fallback: shell

If MCP is unavailable, the CLI is JSON-friendly:

```bash
jarvy tools --index --format json    # Full tool catalog
jarvy doctor --format json           # Environment health
jarvy diff --format json             # Pending changes
jarvy explain <tool> --format json   # Per-tool detail
```

Always pass `--format json` and parse the result. Don't scrape human-readable output.

### Safety rules for AI

1. **Default to dry-run** for installs. Confirm with the user before mutating their system.
2. **Check before installing** with `jarvy_check_tool` — don't reinstall what's already there.
3. **Respect dependencies**. If a user asks for `kubectl`, also offer to install a cluster runtime (the tool's flexible deps).
4. **Never disable rate limits** silently. They exist to prevent runaway agent loops.
5. **Read the audit log** at `~/.jarvy/mcp-audit.log` if behavior seems off.

---

## Mode 2: Generate or Edit `jarvy.toml`

### Authoritative reference

- [Configuration Reference](configuration.md) — every field, every section
- [`jarvy schema`](cli.md#jarvy-schema) — outputs the JSON Schema for editor + agent autocomplete
- [`llms-full.txt`](https://github.com/bearbinary/jarvy/blob/main/llms-full.txt) — single-file flat reference for one-shot agent context
- [`llms.txt`](https://github.com/bearbinary/jarvy/blob/main/llms.txt) — short Q&A optimized for retrieval

### Patterns to follow

**Minimal viable config:**

```toml
[provisioner]
git = "latest"
node = "20"
```

**Team config with role separation:**

```toml
role = "frontend"

[provisioner]
git = "latest"

[roles.base]
tools = ["git", "docker"]

[roles.frontend]
extends = "base"
tools = ["node", "bun"]

[roles.backend]
extends = "base"
tools = ["go", "python"]
```

**Personal email kept out of shared config:**

```toml
[git]
user_name = "Jane Doe"
user_email = { env = "GIT_EMAIL" }
```

**Project bootstrap with language packages:**

```toml
[provisioner]
node = "20"
python = "3.12"

[npm]
typescript = "^5.0"
eslint = "latest"

[pip]
pytest = ">=7.0"
black = "latest"
venv = ".venv"
```

### Anti-patterns to avoid

- **Don't put secrets in `jarvy.toml`.** Use `{ env = "VAR" }` indirection.
- **Don't pin every tool to `latest`.** Pin majors at minimum (`node = "20"`) so version drift is bounded.
- **Don't bypass roles.** If two team members need different tool sets, model it with `[roles.X]`, not by maintaining separate `jarvy.toml` files.
- **Don't redefine tools that exist.** Run `jarvy search <name>` first. Most popular tools are already in the registry.

### Validation loop

After generating a config, validate it:

```bash
jarvy validate                # Schema + value check
jarvy diff                    # Show what would change
jarvy setup --dry-run         # Full plan without execution
```

Fix and iterate before running `jarvy setup` for real.

---

## Mode 3: Modify the Jarvy Codebase

### Required reading

In order:

1. [`CLAUDE.md`](https://github.com/bearbinary/jarvy/blob/main/CLAUDE.md) — project rules + module overview (loaded into Claude Code automatically)
2. [`SKILL.md`](https://github.com/bearbinary/jarvy/blob/main/SKILL.md) — Rust best practices applied in this repo (179 rules)
3. [Architecture](architecture.md) — module map + control flow
4. [Adding Tools](adding-tools.md) — the most common contribution

### Module map (compressed)

```
src/main.rs                Entry point, ~540 lines
src/cli/                   clap definitions
src/commands/              One file per top-level command
src/config.rs              TOML schema
src/remote.rs              Remote config fetch
src/roles/                 Role inheritance
src/tools/                 define_tool! macro + registry + per-tool dirs
src/packages/              npm/pip/cargo package handlers
src/git/                   Git config automation
src/network/               Proxy, TLS, env propagation
src/drift/                 Drift detection + remediation
src/update/                Self-update + rollback
src/logging/               File logging + rotation
src/ticket/                Debug ticket bundles
src/services/              Compose + Tilt
src/env/                   Env vars, secrets, dotenv
src/mcp/                   MCP server
src/telemetry.rs           OTEL pipeline
src/observability/         Telemetry helpers
src/error_codes.rs         Exit codes
build.rs                   Generates tool index JSON
```

Full version with patterns: [Architecture](architecture.md).

### Common tasks → file edits

| Task | Files |
|------|-------|
| Add a tool | `src/tools/<name>/{mod.rs,<name>.rs}` + register in `src/tools/mod.rs` |
| Add a CLI command | `src/cli/args.rs` + `src/cli/subcommands.rs` + `src/commands/<name>.rs` |
| Add a config field | `src/config.rs` struct + default + validation |
| Add a CI provider | `src/ci/detection.rs` + `src/ci/generators/<name>.rs` |
| Add an MCP tool | `src/mcp/tools.rs` (handler) + `src/mcp/server.rs` (registration) |
| Add a default hook | `default_hook` field in the tool's `define_tool!` block |

### Verification before commit

Always run, in order:

```bash
cargo fmt --all
cargo clippy --all-features -- -D warnings
cargo check --verbose
cargo test --verbose -- --show-output
```

The clippy gate is enforced in CI — failures block merge. `correctness` is `deny`-level workspace-wide.

### Conventions

- **Edition 2024**
- **Conventional Commits** (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`)
- **Prefer stdlib + existing deps** over new crates. Adding a dep needs justification in the PR.
- **No `unwrap()`/`expect()` in production paths.** Return `Result` and propagate with `?`. See [`SKILL.md`](https://github.com/bearbinary/jarvy/blob/main/SKILL.md) `err-` rules.

### Testing patterns

Set these env vars in tests that touch external commands:

| Var | Effect |
|-----|--------|
| `JARVY_TEST_MODE=1` | Disable interactive prompts |
| `JARVY_FAST_TEST=1` | Skip external command execution |

Integration tests live in `/tests/`. Use `assert_cmd` for CLI-level testing.

---

## Reference Index

### Single-file references for one-shot agent context

- [`llms.txt`](https://github.com/bearbinary/jarvy/blob/main/llms.txt) — concise Q&A
- [`llms-full.txt`](https://github.com/bearbinary/jarvy/blob/main/llms-full.txt) — full feature reference
- [`jarvy schema`](cli.md#jarvy-schema) — JSON Schema for `jarvy.toml`
- [`jarvy tools --index --format json`](cli.md#jarvy-tools) — full tool catalog

### Per-feature deep dives

- [Configuration](configuration.md)
- [CLI Reference](cli.md)
- [Roles](roles.md)
- [Hooks](hooks.md)
- [Tool Dependencies](tool-dependencies.md)
- [Language Packages](packages.md)
- [Git Configuration](git-config.md)
- [Network & Proxy](network.md)
- [Drift Detection](drift.md)
- [Self-Updating](self-update.md)
- [Logging & Tickets](logging.md)
- [Telemetry](telemetry.md)
- [MCP Server](mcp-server.md)
- [CI/CD Integration](ci-cd.md)
- [Error Codes](error-codes.md)

### Repo metadata

- Repository: <https://github.com/bearbinary/jarvy>
- Issues: <https://github.com/bearbinary/jarvy/issues>
- License: MIT OR Apache-2.0
- Edition: Rust 2024 (rustc ≥ 1.85)
