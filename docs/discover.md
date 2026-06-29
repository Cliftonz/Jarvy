# Auto-discovery (`jarvy discover`)

`jarvy discover` scans the project root for marker files and emits a
suggested `jarvy.toml` so new contributors don't have to guess what
tools the project needs. Drop into any repo and run it — the output is
either a printed suggestion list (default) or a merge straight into
`jarvy.toml` (`--apply`).

> **PRD-044 MVP.** Built-in rules cover the most common ecosystems
> jarvy ships handlers for today (rust, node, python, go, ruby, docker,
> kubectl, helm, terraform, pre-commit, make, just). Custom rule files
> are intentionally deferred — adding a new ecosystem today is one
> entry in `src/discover/rules.rs::default_rules()`.

## Quick start

```bash
cd ~/work/some-project
jarvy discover
```

```text
Project Analysis
================

Detected Technologies:
  rust       1.85.0     (from rust-toolchain.toml)
  docker     latest     (from Dockerfile)
  kubectl    latest     (from k8s)
  pre-commit latest     (from .pre-commit-config.yaml)

Required (would be added):
  rust = "1.85.0"   # detected from rust-toolchain.toml
  docker = "latest"  # detected from Dockerfile

Recommended companions:
  cargo-watch = "latest"   # commonly used with rust
  helm = "latest"          # commonly used with kubectl
  k9s = "latest"           # commonly used with kubectl
  pre-commit = "latest"    # commonly used with pre-commit-config

Run `jarvy discover --apply` to update jarvy.toml.
```

## Flags

| Flag | Behavior |
|------|---------|
| `--file <path>` | Path to the jarvy.toml to read / update. Defaults to `./jarvy.toml`. |
| `--apply` | Write suggestions into the file. Creates it if missing; merges if it exists. |
| `--missing` | Plain `name = "version"` lines only (one per row). Machine-readable but easier to eyeball than JSON. |
| `--rules <path>` | Append a custom rules TOML file to the built-in rule set for this run. Overrides `[discover] rules = "..."` from `jarvy.toml`. |
| `--watch` | Re-run discover whenever a project file changes (notify-driven). 750 ms debounce so editor saves don't flood. Ctrl-C to exit. Falls through to one-shot mode under `--format json`. |
| `--format json` | Full report (detections + required + recommended + already-configured + uninstallable) as JSON. |

## The `[discover]` config block

```toml
# jarvy.toml
[discover]
# Path to a TOML file of extra DetectionRule entries. Custom rules
# APPEND to the built-in set — they can't silence built-in detection.
rules = ".jarvy/discovery-rules.toml"

# Directories to skip during scan (only consulted by the `*.ext` glob
# path today; the root-only walker ignores subdirs by design).
ignore_dirs = ["vendor", "third_party"]
```

The custom rules file mirrors the in-source `DetectionRule` shape:

```toml
# .jarvy/discovery-rules.toml
[[rules]]
name = "rust"           # canonical jarvy tool name
category = "runtime"

[[rules.detect]]
file = "Cargo.toml"

[[rules.detect]]
dir = "src"

[[rules.detect]]
file = "*.yaml"
containing = "kind: RustWorkload"

[rules.version_from]
file = "rust-toolchain.toml"
pattern = 'channel\s*=\s*"([^"]+)"'

[[rules]]
name = "k9s"
category = "ops"
suggests = []
detect = [{ dir = "k8s" }]
```

If the custom rules file fails to parse, `jarvy discover` emits an
advisory warning to stderr and continues with the built-in set — never
hard-fails on a bad rule file.

## Trust posture

`jarvy discover` is dry-run by default. `--apply` is opt-in and the
merge is **append-only**:

- Tools already present under `[provisioner]` are left exactly as
  pinned. A hand-curated `rust = "1.84.0"` survives even when
  rust-toolchain.toml says `1.85.0`.
- New entries are inserted at the end of the existing `[provisioner]`
  block (before the next `[section]`), preserving every comment and
  ordering choice above the insertion point.
- If `[provisioner]` doesn't exist, it's appended at the end of the
  file with a `# Added by jarvy discover` comment.

If the merge would change nothing (no new tools), the file is left
byte-identical.

## Detection rule shape

Each entry in `src/discover/rules.rs::default_rules()` is:

```rust
DetectionRule {
    name: "rust",            // canonical jarvy tool name
    detect: vec![
        File { file: "Cargo.toml" },
        File { file: "rust-toolchain.toml" },
        FileContaining { file: "*.yaml", containing: "kind: Deployment" },
        // ...
    ],
    version_from: Some(VersionSource {
        file: "rust-toolchain.toml",
        pattern: Some(r#"channel\s*=\s*"([^"]+)""#),
    }),
    suggests: vec!["cargo-watch", "cargo-nextest"],
    category: Runtime,
}
```

Three pattern shapes are supported:

- `File { file }` — exact-name or `*.ext` glob match at project root
- `Dir { dir }` — directory exists
- `FileContaining { file, containing }` — bounded 4 KiB scan of the
  matched file for the literal substring. Used today by the kubectl
  rule to catch `kind: Deployment` in bare `*.yaml` files.

The matcher walks only the project root (no subdir descent). This
keeps detection fast on large repos and avoids vendored / `node_modules`
false positives. Add a new ecosystem by appending one entry — no other
code changes needed.

## Version-range narrowing

When `[provisioner]` already pins a semver range that covers the
detected version, discover treats the tool as already-configured —
no re-suggest with an exact pin:

```toml
# jarvy.toml
[provisioner]
node = "^20.0.0"   # ^20.0.0 covers detected `.nvmrc` value of `20`
```

```
$ jarvy discover
…
Already in jarvy.toml:
  node                                    # not re-listed in `Required`
```

If the pinned range does NOT cover the detected version (e.g.
`node = "18"` vs `.nvmrc` says `20`), the tool surfaces in `Required`
with a `(pinned `18` is more lax)` annotation so you can decide
whether to bump.

Falls back to "treat as already-configured" when we can't parse
either side (no detected version, or the pinned spec isn't a valid
semver expression).

## The `uninstallable` bucket

Detected ecosystems jarvy doesn't yet have a first-party installer for
(maven, gradle, dotnet …) surface separately so you see what jarvy
noticed but can't help with:

```
$ jarvy discover
…
Detected but jarvy has no first-party installer for these:
  maven   (from pom.xml)              — no jarvy handler
  gradle  (from build.gradle.kts)     — no jarvy handler
  dotnet  (from global.json)          — no jarvy handler
```

Identical contents available under `uninstallable[]` in `--format json`.

## Continuous discovery (`jarvy setup`)

After `jarvy setup` finishes its install phases, an advisory scan
fires that mirrors `jarvy discover --missing`:

```
$ jarvy setup

…

Tip: `jarvy discover` found 2 additional tool(s) implied by your project files
that aren't yet in [provisioner]:
  - python (detected from pyproject.toml)
  - just (detected from Justfile)
Run `jarvy discover --apply` to pin them.
```

Read-only — never mutates `jarvy.toml`. Quiet in dry-run, when
nothing is new, or when `JARVY_TEST_MODE=1`. Emits a
`discover.setup_advisory` event for dashboard tracking.

## Trust posture

`jarvy discover` is dry-run by default. `--apply` is opt-in and the
merge is **append-only**:

- Tools already present under `[provisioner]` are left exactly as
  pinned. A hand-curated `rust = "1.84.0"` survives even when
  rust-toolchain.toml says `1.85.0`.
- New entries are inserted at the end of the existing `[provisioner]`
  block (before the next `[section]`), preserving every comment and
  ordering choice above the insertion point.
- If `[provisioner]` doesn't exist, it's appended at the end of the
  file with a `# Added by jarvy discover` comment.

If the merge would change nothing (no new tools), the file is left
byte-identical.

Version values extracted from project marker files are sanitized
against a strict allowlist (`[A-Za-z0-9._+~:\-]`, ≤64 chars, BOM
stripped) before they reach the generator, so a hostile
`.python-version` can't inject TOML sections into `jarvy.toml`.

## What's deferred

- `--interactive` confirmation flow (deliberately skipped — the
  dry-run-then-`--apply` two-step covers the same need).

## Telemetry

| Event | When |
|---|---|
| `discover.applied` | `--apply` writes / merges into jarvy.toml |
| `discover.setup_advisory` | `jarvy setup` continuous-discovery phase found new tools |

Both gated through `observability::telemetry_gate::is_enabled()`.
Default-mode read-only scans don't emit.
