# Monorepo workspaces (`jarvy workspace`)

`jarvy workspace` is the read-only inspection surface for monorepo
projects that declare a `[workspace]` block in their root `jarvy.toml`.

> **PRD-047 MVP.** Workspace-aware `jarvy setup --project <name>`
> orchestration is intentionally deferred — surfacing the resolved
> structure first lets users debug inheritance before we add a command
> that mutates the environment based on it.

## Declaring a workspace

```toml
# /repo/jarvy.toml — workspace root
[workspace]
members = ["apps/web", "apps/api", "packages/shared"]

# What sections members inherit from the root config. Empty list is
# treated as ["provisioner"] (the most common case) when inspecting
# resolved tools.
inherit = ["provisioner", "hooks"]

[provisioner]
git = "latest"
docker = "latest"
```

Each member directory may optionally have its own `jarvy.toml` that
adds tools or overrides inherited values:

```toml
# /repo/apps/web/jarvy.toml
[provisioner]
node = "20"
docker = "24.0"     # overrides workspace's docker = "latest"
```

```toml
# /repo/apps/api/jarvy.toml
[provisioner]
go = "1.21"
golangci-lint = "latest"
```

Members without their own `jarvy.toml` inherit the workspace defaults
unchanged.

## CLI

```bash
# Enumerate members + their resolved tool sets
jarvy workspace --file ./jarvy.toml list

# Show one member's resolved config (with inheritance applied + provenance)
jarvy workspace --file ./jarvy.toml show apps/web

# Validate that members exist on disk and their jarvy.toml parses
jarvy workspace --file ./jarvy.toml validate
```

All three subcommands accept `--format json` for AI agents / CI.

### Sample output

```text
$ jarvy workspace --file ./jarvy.toml list
Workspace: /repo
Inherits: provisioner, hooks
Members (3):
  [ok ] apps/web               docker, git, node
  [ok ] apps/api               go, golangci-lint
  [MISS] packages/shared       (uses workspace defaults)
```

```text
$ jarvy workspace --file ./jarvy.toml show apps/web
Project: apps/web
Path:    /repo/apps/web
Config:  /repo/apps/web/jarvy.toml
Inherits sections: provisioner, hooks

Tools (3):
  docker = "24.0" (overridden)
  git = "latest" (inherited)
  node = "20"
```

The `(overridden)` / `(inherited)` annotations come from comparing
the merged provisioner table to the raw root + member tables — gives
a direct answer to "where did this tool come from?"

```text
$ jarvy workspace --file ./jarvy.toml validate
Validating workspace at /repo
  warn: packages/shared: no jarvy.toml (workspace defaults apply)
  2 ok, 1 warning(s), 0 error(s).
```

Validate exits `0` when there are no errors (warnings are advisory)
and `CONFIG_ERROR` (2) when any member's directory is missing or its
jarvy.toml fails to parse.

## Glob member patterns

`members = ["apps/*"]` expands at config-load time to every immediate
subdirectory of `apps/`. Skips `.dotfile` directories. Exact paths
and globs can mix:

```toml
[workspace]
members = [
    "apps/*",          # expand all immediate children
    "packages/shared", # plus this exact member
]
exclude = [
    "apps/legacy",
    "apps/*-deprecated",
]
```

Glob support is intentionally minimal: only `*` (any path component
run, no `/`), no `**`, no `?`, no character classes. The patterns
real monorepos write (`apps/*`, `packages/*-server`) all work; if you
need more, fall back to exact paths.

`exclude = [...]` is applied AFTER expansion using the same matcher
so `apps/*` + `exclude = ["apps/legacy"]` gives you every sibling of
`legacy` minus `legacy` itself.

## `jarvy setup --project <name>`

```bash
# Setup one member explicitly:
jarvy setup --project apps/web

# Auto-detect from cwd:
cd apps/web && jarvy setup

# Same as `--project current`:
jarvy setup --project current
```

The runner reads the member's own `jarvy.toml` (with workspace
inheritance applied). Members WITHOUT a per-member `jarvy.toml` get a
synthesized merged config written to a tempfile so setup still has
something to install against.

Auto-context detection: when invoked WITHOUT `--project`, setup
walks up from cwd to find a workspace root and checks whether cwd
sits inside a declared member. If so, setup scopes implicitly and
prints `Detected workspace member \`apps/web\` — scoping setup to
this member.` to stderr. Pass `--project <name>` (or run from the
workspace root) to override.

The same auto-context applies to `jarvy drift` and `jarvy doctor` —
both honor cwd's enclosing member so `cd apps/web && jarvy drift
check` "just works."

## `jarvy context`

Read-only diagnostic that shows what jarvy thinks the current
execution context is. Useful as a sanity check before running setup
in a new repo.

```bash
$ cd apps/web
$ jarvy context --file /repo/jarvy.toml
Jarvy execution context
=======================
Working dir:   /repo/apps/web
--file arg:    /repo/jarvy.toml
Workspace:     /repo
Root config:   /repo/jarvy.toml
Members (3):
      apps/api
   →  apps/web
      packages/shared
Current member: apps/web

Auto-context:  `jarvy setup` would scope to `apps/web` (override with --project).
Resolved setup file: /repo/apps/web/jarvy.toml
```

Supports `--format json` for AI agents / CI.

## Inheritance semantics

Member configs merge with the root via
`crate::workspace::merge_configs`:

- For sections in `inherit`, member values **completely override** root
  values **except** for `provisioner`, which is merged tool-by-tool
  (member wins on conflict).
- For sections NOT in `inherit`, the member gets only what's in its
  own jarvy.toml.

If `inherit = []` (or omitted), BOTH the `workspace` CLI surface AND
the production setup resolver treat it as `["provisioner"]` — the
common monorepo case (members share the root toolset) works without
explicit config. Routed through `WorkspaceConfig::effective_inherit()`
so CLI display and production setup cannot disagree.

## What's deferred

- Per-member install parallelism — `jarvy setup --project apps/*`
  runs members sequentially today. The existing per-tool parallelism
  (PRD-001 / rayon) stops at the workspace member boundary.
- `[workspace.members.<name>]` inline overrides — alternative to
  per-member `jarvy.toml`. Per-member files cover the case adequately
  for now.
