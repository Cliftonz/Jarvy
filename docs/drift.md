---
title: "Drift Detection - Jarvy"
description: "Detect when a developer's local environment has diverged from the team's expected configuration and remediate it."
---

# Drift Detection

Drift detection catches the moment a developer's environment stops matching the team's `jarvy.toml`. After a successful `jarvy setup`, Jarvy snapshots a baseline. Later runs compare against that baseline and flag tool versions, install methods, and tracked files that have changed.

## Why It Matters

- A teammate manually `brew upgrade`s `node` from 20 → 21 and a build silently breaks
- A new dependency is added to `jarvy.toml` but only half the team re-ran setup
- A `.vscode/settings.json` change wasn't picked up by everyone
- A CI runner image rotates and a tool version drifts

`jarvy drift check` answers: "is this machine still on the team baseline?"

## Enabling Drift

```toml
# jarvy.toml
[drift]
enabled = true
check_on_run = false
track_files = [".vscode/settings.json", "package.json"]
version_policy = "minor"
ignore_tools = ["vim", "neovim"]
allow_upgrades = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Master switch |
| `check_on_run` | bool | `false` | Run drift check before every Jarvy command |
| `track_files` | array | `[]` | Extra files hashed into the baseline |
| `version_policy` | enum | `minor` | How strict version comparison is |
| `ignore_tools` | array | `[]` | Tools excluded from drift comparison |
| `allow_upgrades` | bool | `true` | Treat newer-than-baseline as OK |

## Version Policies

| Policy | Matches | Use when |
|--------|---------|----------|
| `major` | `1.x.x` ↔ `1.y.y` | You only care about API breaks |
| `minor` | `1.2.x` ↔ `1.2.y` | Default; tracks features & patches |
| `patch` | `1.2.3` only | Reproducibility-critical projects |
| `exact` | full string incl. pre-release | Deterministic CI, locked envs |

## Baseline State

State is written to `.jarvy/state.json` after a successful `jarvy setup`:

```json
{
  "version": "1",
  "created_at": "1706086800Z",
  "updated_at": "1706086800Z",
  "config_hash": "sha256:abc123...",
  "tools": {
    "node": {
      "version": "20.10.0",
      "path": "/opt/homebrew/bin/node",
      "install_method": "brew"
    }
  },
  "files": {
    ".vscode/settings.json": "sha256:def456..."
  }
}
```

Commit `.jarvy/state.json` if you want a single shared baseline across the team. Gitignore it if every developer should manage their own.

## Commands

```bash
jarvy drift check                    # Detect drift; exit 1 if found
jarvy drift check --format json      # JSON for CI

jarvy drift status                   # Show baseline summary
jarvy drift status -v                # Show install method + path per tool

jarvy drift accept                   # Treat current state as new baseline
jarvy drift accept --tools node,go   # Accept only these tools

jarvy drift fix                      # Reinstall to match baseline
jarvy drift fix --dry-run            # Preview without changes
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | No drift |
| `1` | Drift detected |
| `2` | No baseline state found — run `jarvy setup` first |

## Common Workflows

### CI Gate

Block PRs that introduce drift:

```yaml
- run: jarvy setup --ci
- run: jarvy drift check --format json
```

### Pre-commit Hook

```bash
#!/bin/sh
jarvy drift check --format json > /dev/null || {
  echo "Environment drift detected. Run: jarvy drift fix"
  exit 1
}
```

### After Upgrading Tools

```bash
brew upgrade node
jarvy drift check         # Reports node version mismatch
jarvy drift accept --tools node   # Adopt new version as baseline
```

## How It Works

1. `jarvy setup` finishes → `EnvironmentState` snapshots tool versions, install methods, paths, and file hashes
2. `jarvy drift check` builds a fresh `EnvironmentState`
3. `DriftDetector` compares baseline vs. current under the configured `version_policy`
4. `DriftReport` lists adds/removes/version-changes/file-changes
5. `DriftFixer` (on `fix`) reinstalls/downgrades tools that have automatic remediation paths

Files are hashed with SHA-256. Tool detection uses the same probes as `jarvy doctor`.

## Module

- Source: `src/drift/`
- Key types: `DriftConfig`, `EnvironmentState`, `ToolState`, `DriftDetector`, `DriftReport`, `DriftFixer`
- See [CLI Reference](cli.md#jarvy-drift) for all flags
