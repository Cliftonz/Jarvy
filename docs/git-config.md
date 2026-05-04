---
title: "Git Configuration - Jarvy"
description: "Automate git identity, commit signing, defaults, line endings, credential helpers, and aliases across the team."
---

# Git Configuration

`[git]` lets `jarvy.toml` codify Git settings the same way it codifies tools. New developers get a correctly configured Git on first `jarvy setup` — no more "I forgot to set my email" PRs.

## Minimal Example

```toml
[git]
user_name = "Jane Doe"
user_email = { env = "GIT_EMAIL" }
default_branch = "main"
pull_rebase = true
```

`user_email = { env = "GIT_EMAIL" }` keeps personal email out of the shared config — each developer sets `GIT_EMAIL` in their shell.

## Full Configuration

```toml
[git]
# Identity
user_name = "Jane Doe"
user_email = { env = "GIT_EMAIL", default = "jane@example.com" }

# Commit signing
signing = true
signing_key = "~/.ssh/id_ed25519.pub"
signing_format = "ssh"            # ssh | gpg, auto-detected from key extension

# Defaults
default_branch = "main"
pull_rebase = true
auto_stash = true
push_autosetup = true
editor = "vim"

# Line endings
autocrlf = "input"                # true | false | input
eol = "lf"

# Credential helper (auto-detected per OS if omitted)
credential_helper = "osxkeychain"

# Scope
scope = "global"                  # global (~/.gitconfig) | local (.git/config)

# Aliases
[git.aliases]
co = "checkout"
br = "branch"
ci = "commit"
st = "status"
lg = "log --oneline --graph --decorate"
```

## ConfigValue Resolution

Any string field accepts three forms:

| Form | Example | Behavior |
|------|---------|----------|
| Plain | `user_name = "Jane"` | Used as-is |
| Env-only | `user_email = { env = "GIT_EMAIL" }` | Reads env at runtime; errors if unset |
| Env + default | `user_email = { env = "GIT_EMAIL", default = "fallback@x.com" }` | Reads env, falls back if unset |

Use the env+default form to keep secrets and personal info out of the shared `jarvy.toml`.

## Signing

Commit signing is auto-detected from the key extension:

| Key | Format detected |
|-----|-----------------|
| `~/.ssh/id_ed25519.pub` | `ssh` |
| `~/.ssh/id_rsa.pub` | `ssh` |
| Any other path | `gpg` |

Override explicitly with `signing_format`:

```toml
signing_format = "gpg"
```

When `signing = true`, Jarvy sets:

- `commit.gpgsign = true`
- `tag.gpgsign = true`
- `gpg.format = ssh|openpgp` based on `signing_format`
- `user.signingkey = <signing_key>`
- For SSH: configures `gpg.ssh.allowedSignersFile` if present

## Credential Helper Defaults

If `credential_helper` is omitted, Jarvy picks per OS:

| OS | Default |
|----|---------|
| macOS | `osxkeychain` |
| Linux | `cache` |
| Windows | `manager-core` |

Override with any helper name accepted by `git config credential.helper`.

## Scope

| Scope | File | Use |
|-------|------|-----|
| `global` (default) | `~/.gitconfig` | Per-developer settings |
| `local` | `.git/config` | Per-repo settings (e.g. work email for a work repo) |

A common pattern: keep `user_name`/`user_email` at scope `local` for a work repo, leave personal global config alone.

## Aliases

```toml
[git.aliases]
co = "checkout"
unstage = "reset HEAD --"
last = "log -1 HEAD"
```

These map directly to `git config --<scope> alias.<name> "<value>"`. Existing aliases are overwritten.

## What Runs

`jarvy setup` invokes `git config --<scope> <key> <value>` for each setting. The order:

1. Identity (`user.name`, `user.email`)
2. Signing config (if enabled)
3. Defaults (`init.defaultBranch`, `pull.rebase`, etc.)
4. Line endings (`core.autocrlf`, `core.eol`)
5. Credential helper
6. Aliases

If `git` itself is missing, the whole `[git]` section is skipped with a warning — install Git first.

## CLI

```bash
jarvy setup           # Applies [git] config
jarvy doctor          # Verifies expected values are set
jarvy diff            # Shows pending git config changes
```

## Module

- Source: `src/git/`
- Files: `config.rs`, `identity.rs`, `signing.rs`, `aliases.rs`, `setup.rs`
- Key types: `GitConfig`, `ConfigValue`, `ConfigScope`, `SigningFormat`, `AutoCrlf`
