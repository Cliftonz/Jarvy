# Replacing Husky with Jarvy

If your repo uses [Husky](https://typicode.github.io/husky/) today,
you have three paths forward, depending on how much migration friction
you want to absorb.

## TL;DR

| Path | Husky stays | New dependency | When to pick it |
|---|---|---|---|
| **Wrap (zero migration)** | Yes | None — Jarvy just installs Husky for you | You like Husky and want a one-command bootstrap for new contributors |
| **Switch to `pre-commit`** | No | `pre-commit` (Python) | Polyglot repo, Husky's per-language friction has cost you |
| **Switch to lefthook** | No | `lefthook` (single Go binary) | You want the fastest hooks + parallel execution, no language runtime |

The rest of this doc walks through each path.

---

## Path 1 — Wrap Husky (zero migration)

Jarvy can drive Husky as a first-class framework. Your `.husky/`
directory and existing hooks stay exactly where they are; the only
change is that `jarvy setup` (and `jarvy hooks install`) now bootstrap
Husky on a fresh clone for you.

### Setup

```toml
# jarvy.toml
[git_hooks]
framework = "husky"     # explicit; otherwise auto-detected from .husky/
auto_install = true     # run `npx husky install` during `jarvy setup`
```

That's it. Existing hooks under `.husky/pre-commit`, `.husky/commit-msg`,
etc. are unchanged. A fresh clone now runs:

```bash
jarvy setup
# → npm install --save-dev husky
# → npx husky install        (writes .husky/_/ + sets core.hooksPath)
```

…and the hooks fire on every `git commit` as before.

### What changes vs. running Husky directly

| Thing | Husky alone | Husky via Jarvy |
|---|---|---|
| `npm install` runs `husky install` | Via `package.json` `prepare` script | Same — Jarvy doesn't remove this; both paths converge |
| New contributor onboarding | `npm install` (assuming they read the README) | `jarvy setup` from a single command in the bootstrap script |
| Husky version updates | `npm install --save-dev husky@latest` by hand | `jarvy hooks update` runs the same thing |
| `jarvy hooks list` | n/a | Enumerates `.husky/*` files (skipping `_/` scaffolding) |
| `jarvy hooks run` | n/a | Runs every `.husky/<name>` script in lex order |
| CI integration | Whatever your pipeline does | `jarvy setup` puts Husky on a known version everywhere |

### Caveats

- **`package.json` is required.** Husky lives in npm dependencies;
  if you don't have a `package.json`, you can't use this path.
  Use path 2 or 3 instead.
- **`npx` must be on PATH.** Surfaced as `HookError::FrameworkNotInstalled`
  if missing. Add Node to `[provisioner]` if your `jarvy.toml` doesn't
  already have it.
- **No autoupdate.** Husky doesn't have a `husky autoupdate` like
  `pre-commit` does — `jarvy hooks update` runs
  `npm install --save-dev husky@latest` and re-installs. Hook scripts
  themselves are your code; Jarvy doesn't touch them.

---

## Path 2 — Switch to `pre-commit`

`pre-commit` is the de-facto polyglot hook framework. It supports
language-specific tooling (rustfmt, black, prettier, …) out of the
box via a YAML config, and the hook implementations live in third-
party repos pinned by `rev`.

### Migration steps

1. **Add `pre-commit` to `jarvy.toml`:**

   ```toml
   [provisioner]
   pre-commit = "latest"

   [git_hooks]
   framework = "pre-commit"
   auto_install = true
   pre_commit.config = ".pre-commit-config.yaml"
   pre_commit.install_hooks = true
   ```

2. **Translate `.husky/<hook-name>` → `.pre-commit-config.yaml`:**

   ```yaml
   # .pre-commit-config.yaml
   repos:
     - repo: https://github.com/pre-commit/pre-commit-hooks
       rev: v4.6.0
       hooks:
         - id: trailing-whitespace
         - id: end-of-file-fixer
         - id: check-yaml

     # Equivalent of `.husky/pre-commit` running `npm run lint`:
     - repo: local
       hooks:
         - id: npm-lint
           name: npm run lint
           entry: npm run lint
           language: system
           pass_filenames: false
   ```

   The `local` repo pattern is the direct replacement for a `.husky/`
   shell script — same intent, just declared in YAML instead of as
   a shell file.

3. **Remove Husky:**

   ```bash
   npm uninstall husky
   rm -rf .husky
   git config --unset core.hooksPath || true   # husky set this; pre-commit doesn't need it
   ```

4. **Bootstrap on fresh clones:**

   ```bash
   jarvy setup
   # → installs pre-commit
   # → runs `pre-commit install --install-hooks`
   ```

### When this wins

- Polyglot repos (`black` + `rustfmt` + `prettier` in the same hook
  pipeline) — `pre-commit`'s pinned-by-rev model handles cross-language
  tooling far better than Husky-as-shell-runner.
- Teams who don't want to require Node on every contributor's machine
  for hook execution.
- CI integration — `pre-commit run --all-files` is the standard
  "verify everything" command across thousands of repos.

### When it doesn't

- JS-heavy monorepos where every contributor already has Node — the
  win from removing Husky is small.
- Repos with lots of custom shell logic in hooks that don't map cleanly
  to the `pre-commit` `local` repo shape.

---

## Path 3 — Switch to `lefthook`

[lefthook](https://github.com/evilmartians/lefthook) is a single Go
binary — no Python, no Node, no language runtime. Hooks are declared
in `lefthook.yml`. Parallel execution by default. Jarvy auto-detects
the `lefthook.yml` marker file but the handler isn't shipped in
this milestone — track [issue #TODO] for the lefthook implementation.

For now, treat lefthook as a path 2 lookalike: install `lefthook` via
your package manager, point `[git_hooks]` at it, and migrate your
`.husky/<name>` scripts into `lefthook.yml`'s `commands` blocks.

---

## Decision tree

```
Are your hooks shell scripts with project-local logic?
├── Yes
│   ├── Polyglot repo? → Path 2 (pre-commit)
│   └── JS-only?       → Path 1 (wrap Husky) — least friction
└── No, hooks are mostly "run npm/cargo/etc. commands"
    ├── Want zero language-runtime deps in hooks? → Path 3 (lefthook)
    └── Otherwise                                  → Path 2 (pre-commit)
```

## Verifying the migration

After migrating, confirm the hooks still fire:

```bash
# Should fire your hooks and exit non-zero on failure.
jarvy hooks run

# Should list every configured hook.
jarvy hooks list

# Status check — useful in CI to assert the framework is wired up.
jarvy hooks status
```

If `jarvy hooks status` reports `installed: no`, run `jarvy hooks install`
once and check `.git/hooks/pre-commit` (or `core.hooksPath`) ends up
pointing where you expect.

---

## Quick reference

```toml
# Path 1 (Husky)
[git_hooks]
framework = "husky"
auto_install = true

# Path 2 (pre-commit)
[git_hooks]
framework = "pre-commit"
auto_install = true
pre_commit.install_hooks = true

# Path 3 (lefthook — not yet shipped)
[git_hooks]
framework = "lefthook"
auto_install = true
```

All three honor the standard `[git_hooks] allow_remote` trust gate —
a remote config (`jarvy setup --from <url>`) cannot land any of these
without explicit opt-in.
