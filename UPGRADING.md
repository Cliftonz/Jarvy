# Upgrading Jarvy

This document covers breaking changes and migration steps between versions.

## Unreleased (development)

## v0.0.5 → v0.1.0

v0.1.0 is the first feature-complete milestone. The public CLI
surface is preserved; the changes below are either fail-closed
defaults that gain an opt-in escape hatch, or invariants tightened
on internal paths. No breaking config changes for typical users.

### `[env.secrets] from_file` paths must resolve under project root or `$HOME`

`from_file` paths that, after symlink-resolving canonicalization,
land outside both the project root (`current_dir`) and `$HOME`
are now refused with `SecretError::PathEscapesProject`. Common
legitimate paths keep working:

- `<project>/.env.secret` ✅
- `~/.aws/credentials` ✅
- `~/.config/myapp/token` ✅
- `/etc/shadow` ❌ (refused)
- `../../etc/passwd` (resolves outside) ❌ (refused)

If your workflow legitimately needs an external path (e.g., a
shared `/var/secrets/...` mount on a build server), set
`JARVY_ALLOW_EXTERNAL_SECRETS=1` to opt in.

```bash
JARVY_ALLOW_EXTERNAL_SECRETS=1 jarvy setup
```

### Pinned third-party installer scripts

`arctl`, `kmcp`, and the Linux fallback path for `ollama` now fetch
their installer scripts at a specific commit and verify the body's
sha256 before exec, matching the existing Homebrew installer. If
upstream rotates the script we ship a hash for, install will fail
fast rather than running new code blindly. Refreshing requires
updating the commit + sha256 constants together — see
`src/tools/{arctl,kmcp,ollama}/definition.rs`.

There is no opt-in to bypass — if the upstream script changes,
update Jarvy or pin manually.

### `[env.vars]` keys validated against POSIX grammar

Keys that don't match `^[A-Za-z_][A-Za-z0-9_]*$` are no longer
written to `~/.bashrc` / `~/.zshrc`. The skipped key is logged
with `event="env.refused_invalid_key"`. If you previously had
keys with hyphens, dots, or leading digits in `[env.vars]`,
rename them to plain identifiers.

### Validator now recognizes more sections

`jarvy validate` previously warned "Unknown configuration section"
for `[npm]`, `[pip]`, `[cargo]`, `[commands]`, `[drift]`, `[git]`,
`[network]`, and `[logging]` even though they're all supported.
These now validate cleanly. No action required — your existing
configs simply produce fewer warnings.

`rust = "stable"` (and other toolchain channel aliases:
`beta`, `nightly`, `lts`, `current`) is now accepted as a valid
version string.

### Lockfile checksum format changed

The lockfile (`jarvy.lock`) checksum algorithm was upgraded from a non-cryptographic hash (`DefaultHasher`) to SHA-256 for integrity verification.

**Impact:** Existing lockfiles will show checksum mismatches after upgrading.

**Migration:** Regenerate your lockfile:

```bash
jarvy lock generate
```

### `--insecure` flag removed

The `--insecure` flag on `jarvy setup --from <url>` was removed. It was never implemented (TLS was always verified). If you had scripts using this flag, remove it.

**Before:**
```bash
jarvy setup --from https://example.com/config.toml --insecure
```

**After:**
```bash
jarvy setup --from https://example.com/config.toml
```

### Config `[commands]` section added

A new optional `[commands]` section lets you configure the interactive menu commands:

```toml
[commands]
run = "npm start"
test = "npm test"
setup = "make dev-setup"
```

Custom commands display a security confirmation prompt before execution. Default commands (`cargo run`, `cargo test`) run without prompting.

### Telemetry `disable` now clears machine fingerprint

Running `jarvy telemetry disable` now also clears the machine fingerprint from `~/.jarvy/config.toml`. Previously, the fingerprint persisted even after disabling telemetry.

### MCP auto-approve preference

When a user selects "Always" during MCP tool install confirmation, the preference is now persisted to `~/.jarvy/config.toml` under `[mcp]`:

```toml
[mcp]
auto_approve_installs = true
```

To reset, set it to `false` or remove the section.

## General Upgrade Steps

1. Update Jarvy:
   ```bash
   jarvy update
   # or
   cargo install jarvy
   ```

2. Regenerate lockfile (if using `jarvy lock`):
   ```bash
   jarvy lock generate
   ```

3. Review `jarvy telemetry status` to confirm your preferences.
