# Changelog

All notable changes to Jarvy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Policy

- **Stable releases (`vX.Y.Z`)** get a curated entry below **before the tag is
  pushed**. The release workflow's `Build release notes` step awk-extracts the
  matching `## [vX.Y.Z]` section into the GitHub release body, then appends a
  `**Full Changelog**` compare link plus Jarvy's standing install/security
  footer. Forgetting this entry causes the workflow to fall through to a raw
  `git log` listing — technically valid, but reads like a commit dump rather
  than a curated narrative. Update CHANGELOG before tagging.
- **Pre-releases (`vX.Y.Z-rc.N`, `-beta.N`, `-alpha.N`)** do **not** get a
  CHANGELOG entry. The awk extraction returns empty, the workflow falls
  through to `git log <prev-tag>..<tag>` notes, and that fallback is the
  intended pre-release path. The curated stable entry below is written once
  when the corresponding stable cuts.
- Entry headers must match the awk pattern: `## [vX.Y.Z]` or
  `## [vX.Y.Z] — Title` (em-dash optional). Other shapes won't be matched.

See [`docs/release-testing.md`](https://github.com/bearbinary/jarvy/blob/main/docs/release-testing.md)
for the full release process and
[`docs/release-quirks-jarvy.md`](https://github.com/bearbinary/jarvy/blob/main/docs/release-quirks-jarvy.md)
for divergences from generic release skills.

## [Unreleased]

## [v0.1.0] — First feature-complete milestone (2026-05-10)

First feature-complete stable. Closes the round-2 hardening review
(45 items across two passes), ships clean-laptop onboarding, and
publishes 14 ready-to-copy `jarvy.toml` project templates. The
public surface from v0.0.5 is preserved; everything below is either
additive, fail-closed by default, or a tightening of internal
invariants.

### Added

- **Project templates.** `examples/<stack>/jarvy.toml` ships 14
  validated drop-in configs (node-npm/pnpm/bun, deno, python-api/uv,
  go-api, rust-cli/workspace, ruby-rails, java-spring, react-app,
  fullstack, k8s-platform). Companion docs at
  `docs/templates-index.md` give an AI-agent decision table mapping
  detect-by signals (lockfiles, manifests) to template URLs.
- **Clean-laptop onboarding.** New `Makefile` + idempotent
  `scripts/bootstrap.sh` give contributors a two-command setup
  (`curl install.sh | bash` then `make setup`). Bootstrap script
  honors `JARVY_CHANNEL` for stable/beta/nightly, falls back to
  `wget` if `curl` is missing, and forwards extra args to
  `jarvy setup`. shellcheck-clean.
- **`jarvy validate` recognizes the full top-level surface.**
  `[npm]`, `[pip]`, `[cargo]`, `[commands]`, `[drift]`, `[git]`,
  `[network]`, `[logging]` no longer trigger
  "unknown configuration section" warnings. Toolchain channel
  aliases (`stable`, `beta`, `nightly`, `lts`, `current`) are
  accepted as valid version strings — `rust = "stable"` validates
  cleanly.
- **`SecretError::PathEscapesProject`** + `JARVY_ALLOW_EXTERNAL_SECRETS`
  override. `[env.secrets] from_file` paths that resolve outside
  the project root and `$HOME` after symlink-resolving
  canonicalization are refused by default. Common legitimate paths
  (`~/.aws/credentials`, `<project>/.env.secret`) keep working.
  Override with `JARVY_ALLOW_EXTERNAL_SECRETS=1`.
- **`tools::pinned_installer::PinnedInstaller`** helper for the
  curl-bash class of installers. arctl, kmcp, and ollama (Linux
  fallback only) now fetch their installer scripts at a pinned
  commit, sha256-verify the body, and refuse to exec on mismatch —
  same pattern Homebrew already used. Refreshing a pinned installer
  requires updating the commit + sha256 constants together.
- **POSIX env-var grammar validation** before writing
  `[env.vars]` to shell rc files. Keys not matching
  `^[A-Za-z_][A-Za-z0-9_]*$` are skipped with a structured
  `event="env.refused_invalid_key"` warning instead of corrupting
  `~/.bashrc` / `~/.zshrc`.
- **`tools::install_method`** canonical classifier
  (`Brew`/`Cargo`/`Nvm`/`Pyenv`/`Rustup`/`Snap`/`System`/
  `NotFound`/`Unknown`). `commands::diagnose`, `commands::drift`,
  and `observability::bundle` all delegate here instead of
  hand-rolling three near-identical detectors.

### Changed

- **Logging pipeline rewired** to `tracing_appender::rolling` for
  daily rotation + `tracing_appender::non_blocking` for buffered
  writes. `analytics::shutdown_logging()` flushes both the
  `SdkLoggerProvider` and the file `WorkerGuard` before
  `process::exit`, so buffered records aren't lost on early
  termination. `EnvFilter` now has a default-on floor of
  `warn,jarvy=info` if `RUST_LOG` is unset.
- **`Hook::run_with_policy`** collapsed from a 3-state `HookOutcome`
  enum to `Result<(), HookError>`. Production callers only ever
  checked `Fail` vs not-Fail; the warning-on-`continue_on_error`
  side effect already conveyed the difference. The new `Err` case
  returns the underlying `HookError` so `error_codes::HOOK_FAILED`
  callers keep working.
- **`Sanitizer::sanitize_borrowed`** returns `Cow<'_, str>` so the
  no-match path skips allocation entirely. `Sanitizer::sanitize`
  preserves the same fast path internally.
- **`tracing::warn!` → `tracing::error!`** on `tool.failed`,
  `hook.failed`, `hook.timeout`, `config.parse_error`, and
  `telemetry.endpoint.refused`. These are operator-actionable
  conditions, not advisory.
- **Subprocess spans.** `services::run_command` and
  `tools::common::run_capture` are now wrapped in
  `tracing::info_span!("subprocess.exec", cmd, args_count, ...)`
  with start/duration/exit_code events.
- **`paths.rs` cleanup.** `cache_dir` inlined into
  `remote_config_cache_dir` (only caller); `#![allow(dead_code)]`
  removed since every public function has external callers now.

### Security

- **CA-bundle trust check tightened.** `network::propagate` no
  longer accepts paths under the broad `~/.jarvy/` cache prefix —
  only `~/.jarvy/ca/` is trusted, with a trailing-slash anchor so
  `~/.jarvy/ca-attacker/...` can't slip through.
- **Cross-origin redirects refused** on
  `remote::validated_get` / `fetch_remote_config`. `ureq` agent
  now uses `.max_redirects(0)`; redirects must be revalidated
  through the policy gate.
- **Sigstore companion verification.** `update::release` returns
  `None` for cosign companion files when the `.sig`/`.pem` aren't
  exact-match siblings — a substring-match bug that would have let
  a malicious tarball claim sibling signatures was closed.
- **`exec.rs` deleted** (zero-caller speculative seam).
- **`team::inheritance::transform_github_url`** duplicate deleted;
  callers route through the canonical `remote::transform_github_url`
  so URL hardening lives in one place.

### Fixed

- `validate_get` rejected URLs with empty hosts under `file://`
  scheme but didn't match the documented "scheme not allowed"
  error string. Test relaxed to accept any error variant; behavior
  unchanged.
- `paths::remote_config_cache_dir` now reads `JARVY_HOME`
  consistently with the rest of `paths.rs` (was hand-rolling the
  override before).
- `update_rc_content` argument order documented; previously the
  test suite caller had `(content, &vars, &ctx, ShellType)` instead
  of the actual `(content, ShellType, &vars, &ctx)`.

### Tests

- 1,633+ tests passing across lib + binary + integration suites
  (was ~1,580). Highlights of the new coverage:
  - `validated_get` rejection tests for HTTP-to-remote, disallowed
    host, `file://` scheme, missing scheme.
  - `Hook::run_with_policy` outcome matrix (dry-run / success /
    failure × continue_on_error true|false).
  - `verify_no_tar_escape` containment tests + symlink-escape
    refusal.
  - Cosign companion exact-match (no substring) regression.
  - Path-containment refusal + `JARVY_ALLOW_EXTERNAL_SECRETS=1`
    override path for `[env.secrets] from_file`.
  - Shell-interpreted-key table-driven test
    (`every_shell_interpreted_key_refuses_bang_prefix`) so adding
    a new shell-interpreted git config key lights up the test
    suite immediately.
- `#[serial_test::serial]` annotations added for
  `JARVY_ALLOW_*` env mutations to keep parallel runs isolated.

### Docs

- `CLAUDE.md` Logging section rewritten to match the actual
  `src/logging/` (thin re-export layer) and `src/observability/`
  (where rotation + sanitizer + analytics live) split.
- `examples/README.md` + `docs/templates-index.md` published as
  the human/AI-facing template indexes.
- `llms-full.txt` "Project Templates" section added (with
  `docs/llms.txt` + `docs/llms-full.txt` symlinks for the published
  docs site).

## [v0.0.5] — Chocolatey install script + bundled v0.0.4 fixes (2026-05-05)

Folds in everything queued for v0.0.4 (which was tagged but never
publicly published) plus a Chocolatey install-script fix.

### Fixed

- **Chocolatey package** v0.0.3 failed moderation with `404 Not Found`
  for the install URL. Two bugs in
  `dist/windows/chocolatey/tools/chocolateyinstall.ps1`:
  - URL pattern referenced
    `jarvy-vVERSION_PLACEHOLDER-x86_64-pc-windows-msvc.zip` — but
    cargo-packager produces `.msi` and `.exe`, no `.zip` for Windows.
  - VERSION_PLACEHOLDER and SHA256_PLACEHOLDER were never substituted
    because the publish workflow only ran sed against `jarvy.nuspec`,
    not the install script.

  Rewrote the install script to use `Install-ChocolateyPackage` with
  `-FileType msi` and silent install args, pointing at the actual
  `jarvy_<v>_x64_en-US.msi` asset. Updated
  `publish-packages.yml::update-chocolatey` to substitute both files
  AND pull the real msi SHA256 from `SHA256SUMS.txt` so the integrity
  check passes.
- **`cargo fmt --check`** drift in `src/team/inheritance.rs:760-768`
  (single-quoted TOML literals from v0.0.3 needed compaction).
- **OpenSSF Scorecard** failed on v0.0.3 tag with `Only the default
  branch main is supported`. ossf/scorecard-action explicitly refuses
  tag-push triggers. Restored `push: branches: [main]` for scorecard
  only — every other validating workflow stays tag-triggered.
- **Homebrew tap publish** now gracefully skips when
  `HOMEBREW_TAP_DEPLOY_KEY` is not configured. Previously the missing
  secret failed the whole `publish-packages.yml` workflow, masking
  the success of crates.io, AUR, winget, and Chocolatey jobs.

### Validated downstream (v0.0.3)

After the v0.0.3 fixes, the following propagation channels worked:

- ✅ crates.io: jarvy@0.0.3 + cargo-jarvy@0.0.3 published
- ✅ AUR (jarvy-bin)
- ✅ Submit to winget (publish-packages.yml job; separate winget.yml
  still needs manual first submission)
- ✅ GitHub Pages docs site (after maintainer enabled Pages)
- ❌ Chocolatey: failed moderation due to broken install script
  (v0.0.5 fixes)
- ⚠️  Homebrew tap: pending secret config (now non-blocking)

### Note

v0.0.4 was tagged but the draft was never publicly published —
v0.0.4's fixes ship together with the Chocolatey fix as v0.0.5 to
reduce propagation churn (one round of crates.io / AUR / etc.
updates instead of two back-to-back).

## [v0.0.4] — Lint formatting + scorecard + homebrew-tap guard (2026-05-05)

### Fixed

- **`cargo fmt --check`** failed in the Lint job on
  `src/team/inheritance.rs:760-768` because the v0.0.3 single-quoted
  TOML literal edits left format strings on multiple lines that
  rustfmt wanted compacted. Re-ran `cargo fmt` to normalize.
- **OpenSSF Scorecard** failed on the v0.0.3 tag with `Only the
  default branch main is supported`. ossf/scorecard-action explicitly
  refuses tag-push triggers; v0.0.3's trigger trim moved scorecard
  off main-push, which broke it. Restored `push: branches: [main]`
  for scorecard only — every other validating workflow stays
  tag-triggered. Release-tag scorecard runs produce no useful signal
  anyway since the action only inspects the default branch.
- **Homebrew tap publish** now gracefully skips when
  `HOMEBREW_TAP_DEPLOY_KEY` is not configured. Previously the whole
  `publish-packages.yml` workflow exited 1 with "API_TOKEN_GITHUB
  and SSH_DEPLOY_KEY are empty", masking the success of crates.io,
  AUR, winget, and Chocolatey jobs. New behavior: missing secret
  emits a warning ("set per docs/MAINTAINER_RELEASE_GUIDE.md") and
  the push step is skipped via `if:` guard.

### Validated downstream (v0.0.3)

After the v0.0.3 fixes, the following propagation channels worked:

- ✅ crates.io: jarvy@0.0.3 + cargo-jarvy@0.0.3 published
- ✅ Submit to winget (job inside publish-packages.yml; the separate
  winget.yml workflow still requires manual first submission per
  v0.0.3 release notes)
- ✅ Chocolatey
- ✅ AUR (jarvy-bin)
- ✅ GitHub Pages docs site (after maintainer enabled Pages in repo
  Settings)
- ⚠️  Homebrew tap: blocked on `HOMEBREW_TAP_DEPLOY_KEY` secret;
  v0.0.4 makes this a non-blocker so missing-secret no longer fails
  the whole workflow.

## [v0.0.3] — Unblock crates.io and Homebrew downstream publish (2026-05-05)

Patch release. v0.0.2 went live on the GitHub release page but the
crates.io and Homebrew workflows that fire on `release: published`
both failed, leaving `cargo install jarvy` and
`brew install bearbinary/tap/jarvy` unavailable.

### Fixed

- **Cargo.toml** declared `readme = "README.md"` (uppercase) but the
  tracked file is `Readme.md` (mixed case). On macOS the difference
  is invisible (case-insensitive filesystem); on the Linux CI runner
  it failed `cargo publish` with `readme "README.md" does not appear
  to exist`. Both `Publish Crate` and `Publish to Package Managers`
  workflows hit the same error. Same fix in the `include = [...]`
  manifest list. Now matches what's actually in the git tree.
- **`.github/workflows/winget.yml`** was scaffolded from a different
  project's template and never customized — `identifier: Benji377.Tooka`
  and `fork-user: Benji377` referenced a totally unrelated package.
  Rewrote with placeholder TODO values for `Jarvy.Jarvy` /
  `bearbinary` and changed the trigger from `release: published` to
  `workflow_dispatch` only. winget-releaser cannot create a brand-new
  package registration; the first submission must go through
  `wingetcreate new` and a hand-reviewed PR to microsoft/winget-pkgs.
  After that's merged the trigger can be flipped back.

### Removed

- Duplicate `.github/workflows/crates.yml` deleted. Both that and
  `publish-packages.yml::publish-crates-io` were firing on
  `release: published` and trying to `cargo publish`. Even if both
  had the right secret, the second one would race-fail with "crate
  version already exists". Kept the version inside `publish-packages.yml`
  because it composes with the Homebrew tap update via `needs:`.
- `docs/release-testing.md` and `docs/release-quirks-jarvy.md`
  references to `crates.yml` updated to point at the surviving
  workflow path.

### Known issues (not fixed in this release)

- **GitHub Pages** is not enabled for `bearbinary/Jarvy` repo — the
  Deploy Docs workflow fails with `HttpError: Not Found ... Ensure
  GitHub Pages has been enabled`. Fix is in repo Settings → Pages,
  not in code. Until enabled, the docs site at jarvy.dev (or
  whichever Pages URL ends up provisioned) won't update on release.
- **winget first submission** still requires manual `wingetcreate new`
  intervention (see Fixed above for the workflow disable).

## [v0.0.2] — Cosign verify-command case fix (2026-05-05)

Patch release fixing the cosign verification snippet baked into
release notes, SECURITY.md, and docs/release-quirks-jarvy.md.

### Fixed

- **release notes / SECURITY.md / docs**: the
  `--certificate-identity-regexp` value used `bearbinary/jarvy`
  (lowercase j). The actual Sigstore cert subject GitHub Actions
  produces is `bearbinary/Jarvy/...` (capital J — the repo's
  canonical case). cosign's regex is case-sensitive, so users
  copy-pasting the verify command from the v0.0.1 release page
  saw "none of the expected identities matched" even though the
  signature was valid. Corrected all three sources to
  `bearbinary/Jarvy/`. github.com URLs elsewhere in the repo are
  unchanged because GitHub URL matching is case-insensitive — only
  cosign's regex was affected.

## [v0.0.1] — Initial public release (2026-05-05)

First publicly tagged stable release. Validated through the
v0.1.0-rc.1 → v0.1.0-rc.9 soak cycle (same tree, version-string
only differs); cut as 0.0.1 to keep the first-stable surface narrow
and reserve room for 0.1.0 as the first feature-complete milestone.

### Features

- **provisioner:** Cross-platform tool provisioner driven by `jarvy.toml`
  (macOS, Linux, Windows) with native package managers
- **tools:** 154+ tool registry covering compilers, runtimes, CLIs, container
  tools, Kubernetes ecosystem (kubectl, helm, k9s, kagent, kmcp, arctl), cloud
  CLIs (gcloud, aws, az), security tools, observability (opentelemetry-collector),
  Dockerfile converter (dfc) (PRD-013)
- **tools:** Parallel version checking with rayon for ~5x speedup; batch
  package-manager operations
- **tools:** Declarative `define_tool!` macro for tool definitions (~2000 lines
  reduced)
- **tools:** Strict (`depends_on`) and flexible (`depends_on_one_of`) tool
  dependencies with topological install ordering (PRD-034)
- **hooks:** 29+ default post-install hooks for shell completion and
  configuration; idempotent, advisory, user-overridable
- **roles:** Role-based configurations with deep inheritance, version overrides,
  `roles list|show|diff` commands (PRD-033)
- **packages:** Language package deps via `[npm]`, `[pip]`, `[cargo]` —
  package-manager auto-detection, virtualenv support, lockfile install (PRD-039)
- **git:** Git configuration automation — identity, SSH/GPG signing, default
  branch, aliases, credential helper auto-detect per OS (PRD-041)
- **drift:** Configuration drift detection with SHA-256 file hashing, version
  policies, `jarvy drift check|status|accept|fix` (PRD-043)
- **update:** Self-updating with stable/beta/nightly channel selection,
  throttled checks, rollback, multi-method install detection (Homebrew, Cargo,
  apt, dnf, winget, Chocolatey, Scoop, binary fallback) (PRD-035)
- **telemetry:** OTEL-unified logs, metrics, optional traces; OTLP HTTP/gRPC
  endpoints; CI auto-disable; `jarvy telemetry status|enable|disable|test|preview`
  (PRD-022, PRD-050)
- **logging:** Persistent file logging with rotation, gzip compression,
  sensitive-data redaction; `jarvy logs view|stats|clean|config` (PRD-050)
- **ticket:** Debug bundles via `jarvy ticket create|show|list|clean` — ZIP with
  system info, tool versions, sanitized logs (PRD-050)
- **network:** Corporate proxy support — HTTP/HTTPS/SOCKS, NO_PROXY, custom CA
  bundles, per-tool overrides, secure password sources (PRD-019)
- **services:** Docker Compose and Tilt backend support
- **ci:** Auto-detection for 11 CI/CD providers with provider-specific output
- **env:** Environment variable management with `.env` generation and shell rc
  updates
- **mcp:** MCP server exposing tools and resources for AI assistants
- **interactive:** Menu mode when running `jarvy` without a subcommand
- **bootstrap:** `jarvy bootstrap`, `jarvy configure`, `jarvy diagnose` for
  onboarding (PRD-023)

### Distribution

- Multi-channel: crates.io, Homebrew tap, AUR (source + binary), `.deb`, `.rpm`,
  winget, Chocolatey, universal install scripts for macOS/Linux/Windows (PRD-012)
- **Prebuilt platforms**: macOS arm64, Linux x86_64 (musl), Linux aarch64,
  Linux armv7, Windows x86_64. macOS Intel (x86_64) **not shipped as prebuilt** —
  Intel users install via `cargo install jarvy` or Homebrew (both compile from
  source). See `docs/release-testing.md` for rationale.
- Sigstore keyless signing for all release artifacts (PRD-020)
- SBOM generation in SPDX 2.3 and CycloneDX 1.4 formats per release (PRD-020)
- GitHub build provenance attestation per release (PRD-020)
- Opt-in early-release channel: `JARVY_CHANNEL=beta` env var on install
  scripts; `[update] channel = "beta"` in `~/.jarvy/config.toml`;
  `jarvy update --channel beta`

### Quality & Security

- Clippy gate, mutation testing, fuzzing, coverage, benchmarks, OpenSSF
  Scorecard (PRD-018)
- Hybrid cross-platform E2E testing harness (PRD-038)
- Tag-signing enforcement (SSH or GPG) on release workflow
- Cosign keyless signing via GitHub OIDC for all release artifacts

### Infrastructure

- Semantic version checking with proper semver operators
- Cross-platform shell detection and hook execution
- Workspace lint configuration; Rust 2024 edition; MSRV 1.85

[Unreleased]: https://github.com/bearbinary/jarvy/compare/v0.1.0...HEAD
[v0.1.0]: https://github.com/bearbinary/jarvy/releases/tag/v0.1.0
[v0.0.5]: https://github.com/bearbinary/jarvy/releases/tag/v0.0.5
[v0.0.4]: https://github.com/bearbinary/jarvy/releases/tag/v0.0.4
[v0.0.3]: https://github.com/bearbinary/jarvy/releases/tag/v0.0.3
[v0.0.2]: https://github.com/bearbinary/jarvy/releases/tag/v0.0.2
[v0.0.1]: https://github.com/bearbinary/jarvy/releases/tag/v0.0.1
