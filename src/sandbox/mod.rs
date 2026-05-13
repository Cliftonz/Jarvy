//! AI sandbox + container environment detection.
//!
//! Mirror of `crate::ci` for the broader class of non-interactive
//! execution environments — AI agent sandboxes (Claude Code, Cursor,
//! e2b, Modal, Daytona, Replit), long-running container envs
//! (Codespaces, Gitpod, devcontainers), and a generic-container
//! fallback for anything that mounts `/.dockerenv` without a TTY.
//!
//! The CI detector handles continuous-integration runners — a strict
//! subset of "sandbox-ish" environments. This module subsumes that
//! subset via `is_seamless()`, which returns true for both CI and
//! sandbox cases. Callers asking "should I avoid prompts / disable
//! telemetry / skip update checks" should use `is_seamless()`.
//!
//! Detection is environmental (env vars + filesystem stat). No
//! subprocess fork; total cost under a millisecond. Designed and
//! documented in PRD-053.

use std::env;
use std::io::IsTerminal;
use std::sync::OnceLock;

/// Recognized sandbox / container providers.
///
/// Order matches detection priority in `detect_provider()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxProvider {
    /// GitHub Codespaces (`CODESPACES=true`)
    Codespaces,
    /// Gitpod (`GITPOD_WORKSPACE_ID` set)
    Gitpod,
    /// VS Code Remote Containers / devcontainer
    /// (`REMOTE_CONTAINERS=true` or `DEVCONTAINER=true`)
    Devcontainer,
    /// Replit Agent (`REPL_ID` set)
    Replit,
    /// e2b sandbox (`E2B_SANDBOX_ID` set)
    E2b,
    /// Modal container (`MODAL_TASK_ID` set)
    Modal,
    /// Daytona workspace (`DAYTONA_WS_ID` set)
    Daytona,
    /// Claude Code (`CLAUDECODE=1` or `CLAUDE_CODE_ENTRYPOINT` set)
    ClaudeCode,
    /// Cursor background agent (`CURSOR_AGENT=1`)
    Cursor,
    /// Generic container — `/.dockerenv` exists and stdin is not a
    /// TTY. Fallback for unrecognized container runtimes.
    GenericContainer,
}

impl SandboxProvider {
    /// Human-readable provider name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Codespaces => "GitHub Codespaces",
            Self::Gitpod => "Gitpod",
            Self::Devcontainer => "Devcontainer",
            Self::Replit => "Replit",
            Self::E2b => "e2b",
            Self::Modal => "Modal",
            Self::Daytona => "Daytona",
            Self::ClaudeCode => "Claude Code",
            Self::Cursor => "Cursor",
            Self::GenericContainer => "container",
        }
    }
}

impl std::fmt::Display for SandboxProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Whether Jarvy can actually install tools in this sandbox.
///
/// Probed lazily on first call to `install_capability()` and cached
/// for the lifetime of the process — the rootfs and the package
/// managers on PATH do not change mid-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCapability {
    /// Sudo available OR a user-scope package manager on PATH.
    Full,
    /// Read-only rootfs OR no install path available; switch to
    /// verify-only (`jarvy doctor`) instead of attempting installs.
    /// Reason is preserved so logs/tickets can explain *which*
    /// probe tripped.
    VerifyOnly(VerifyOnlyReason),
}

/// Why the install-capability probe declared verify-only mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOnlyReason {
    /// `~/.jarvy/` could not be resolved (no home dir, hardened
    /// `JARVY_HOME` rejected).
    NoJarvyHome,
    /// Could not write into `~/.jarvy/` — read-only rootfs, EROFS,
    /// EACCES, etc.
    ReadOnlyRoot,
    /// No user-scope package manager on PATH (brew/cargo/winget/scoop)
    /// **and** `sudo -n true` did not succeed. Tool installs need one
    /// of those paths.
    NoInstallPath,
    /// Forced via `JARVY_FORCE_VERIFY_ONLY=1` test override.
    Forced,
}

impl std::fmt::Display for VerifyOnlyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NoJarvyHome => "no_jarvy_home",
            Self::ReadOnlyRoot => "read_only_root",
            Self::NoInstallPath => "no_install_path",
            Self::Forced => "forced",
        };
        f.write_str(s)
    }
}

/// Result of `detect()`.
#[derive(Debug, Clone)]
pub struct SandboxEnvironment {
    /// Which sandbox flavor was detected.
    pub provider: SandboxProvider,
    /// True if forced via `JARVY_SANDBOX=1` rather than auto-detected.
    pub forced: bool,
}

/// Detect the current sandbox environment.
///
/// Precedence:
/// 1. `JARVY_SANDBOX=0` → `None` (disable detection)
/// 2. `JARVY_SANDBOX=1` → forced generic-container (or whatever
///    named provider also matches)
/// 3. Named providers in table order
/// 4. Generic-container fallback (`/.dockerenv` AND non-TTY stdin)
/// 5. Otherwise `None`
///
/// Cached per process: env vars and `/.dockerenv` do not change
/// mid-run. The first call emits a `sandbox.detected` tracing event
/// so support tickets show *why* seamless mode activated.
pub fn detect() -> Option<SandboxEnvironment> {
    cached_detect()
}

#[cfg(not(test))]
fn cached_detect() -> Option<SandboxEnvironment> {
    static CACHE: OnceLock<Option<SandboxEnvironment>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let result = detect_uncached();
            if let Some(env_) = &result {
                tracing::info!(
                    event = "sandbox.detected",
                    provider = %env_.provider,
                    forced = env_.forced,
                    "seamless mode active"
                );
            } else if env::var("JARVY_SANDBOX").as_deref() == Ok("0") {
                tracing::info!(
                    event = "sandbox.detect.suppressed",
                    reason = "JARVY_SANDBOX=0",
                    "sandbox detection disabled by user override"
                );
            }
            result
        })
        .clone()
}

// Tests must see fresh state per call; the `with_env` helper sets
// env vars then expects `detect()` to re-evaluate. Skip caching
// under `cfg(test)`.
#[cfg(test)]
fn cached_detect() -> Option<SandboxEnvironment> {
    detect_uncached()
}

fn detect_uncached() -> Option<SandboxEnvironment> {
    if env::var("JARVY_SANDBOX").as_deref() == Ok("0") {
        return None;
    }

    let forced = env::var("JARVY_SANDBOX").as_deref() == Ok("1");
    let provider = detect_provider();

    match (forced, provider) {
        (true, Some(p)) => Some(SandboxEnvironment {
            provider: p,
            forced: true,
        }),
        (true, None) => Some(SandboxEnvironment {
            provider: SandboxProvider::GenericContainer,
            forced: true,
        }),
        (false, Some(p)) => Some(SandboxEnvironment {
            provider: p,
            forced: false,
        }),
        (false, None) => None,
    }
}

/// True if a sandbox is detected. Does *not* include CI environments
/// — for the broader "should I avoid prompts" question, use
/// `is_seamless()`.
pub fn is_sandbox() -> bool {
    detect().is_some()
}

/// True if Jarvy should behave non-interactively: no prompts, quiet
/// output by default, telemetry off unless explicitly opted in,
/// update checks suppressed. Returns true for **either** CI **or**
/// sandbox detection.
///
/// This is the canonical predicate for unattended-mode decisions.
/// Migrate ad-hoc `env::var("CI").is_ok()` checks to this.
pub fn is_seamless() -> bool {
    is_sandbox() || crate::ci::is_ci()
}

/// Like `is_seamless()`, but excludes *forced* sandbox detection.
/// Telemetry and update auto-disable should use this so an attacker
/// who can set `JARVY_SANDBOX=1` in a victim's shell cannot silence
/// security-patch updates or anomaly reports. CI detection and
/// auto-detected sandboxes still flip the gate.
pub fn is_seamless_auto() -> bool {
    let sandbox_auto = detect().map(|e| !e.forced).unwrap_or(false);
    sandbox_auto || crate::ci::is_ci()
}

/// Install capability for this process. Probed once on first call.
pub fn install_capability() -> InstallCapability {
    static CACHED: OnceLock<InstallCapability> = OnceLock::new();
    *CACHED.get_or_init(probe_install_capability)
}

fn detect_provider() -> Option<SandboxProvider> {
    // Order: most-specific signal first, generic fallback last.

    if env::var("CODESPACES").as_deref() == Ok("true") || env::var("CODESPACE_NAME").is_ok() {
        return Some(SandboxProvider::Codespaces);
    }

    if env::var("GITPOD_WORKSPACE_ID").is_ok() {
        return Some(SandboxProvider::Gitpod);
    }

    if env::var("REMOTE_CONTAINERS").as_deref() == Ok("true")
        || env::var("DEVCONTAINER").as_deref() == Ok("true")
    {
        return Some(SandboxProvider::Devcontainer);
    }

    if env::var("REPL_ID").is_ok() {
        return Some(SandboxProvider::Replit);
    }

    if env::var("E2B_SANDBOX_ID").is_ok() {
        return Some(SandboxProvider::E2b);
    }

    if env::var("MODAL_TASK_ID").is_ok() {
        return Some(SandboxProvider::Modal);
    }

    if env::var("DAYTONA_WS_ID").is_ok() {
        return Some(SandboxProvider::Daytona);
    }

    if env::var("CLAUDECODE").as_deref() == Ok("1") || env::var("CLAUDE_CODE_ENTRYPOINT").is_ok() {
        return Some(SandboxProvider::ClaudeCode);
    }

    if env::var("CURSOR_AGENT").as_deref() == Ok("1") {
        return Some(SandboxProvider::Cursor);
    }

    // Generic-container fallback. Requires BOTH `/.dockerenv` present
    // AND stdin not a TTY — a developer running `jarvy` inside a
    // container they shelled into still has a TTY and gets normal
    // interactive behavior.
    if is_generic_container(
        std::path::Path::new("/.dockerenv").exists(),
        std::io::stdin().is_terminal(),
    ) {
        return Some(SandboxProvider::GenericContainer);
    }

    None
}

/// Pure predicate behind the generic-container fallback. Both signals
/// are required so that a developer who `docker exec -it` into a
/// container is not trapped in seamless mode just by virtue of being
/// inside any container. Extracted so the AND-vs-OR distinction can
/// be unit-tested without faking `/.dockerenv` or stdin.
fn is_generic_container(dockerenv_present: bool, stdin_is_tty: bool) -> bool {
    dockerenv_present && !stdin_is_tty
}

fn probe_install_capability() -> InstallCapability {
    // Test override for integration tests.
    if env::var("JARVY_FORCE_VERIFY_ONLY").as_deref() == Ok("1") {
        return InstallCapability::VerifyOnly(VerifyOnlyReason::Forced);
    }

    // 1. Read-only rootfs probe. We write into `~/.jarvy/` (a path
    //    Jarvy owns anyway); if that path is unwritable the install
    //    pipeline (state.json, logs, baseline) is doomed regardless
    //    of pkg-manager availability.
    //
    //    Use a per-PID filename + `create_new(true)` (O_CREAT|O_EXCL)
    //    so a pre-staged symlink at the probe path errors out
    //    instead of being followed and clobbered. See PRD-053
    //    security review F3.
    let Ok(home) = crate::paths::jarvy_home() else {
        tracing::warn!(
            event = "sandbox.capability.probe_failed",
            probe = "jarvy_home",
            reason = "no_home_dir"
        );
        return InstallCapability::VerifyOnly(VerifyOnlyReason::NoJarvyHome);
    };
    if let Err(e) = std::fs::create_dir_all(&home) {
        tracing::warn!(
            event = "sandbox.capability.probe_failed",
            probe = "create_jarvy_home",
            path = %home.display(),
            err = %e
        );
        return InstallCapability::VerifyOnly(VerifyOnlyReason::ReadOnlyRoot);
    }
    let probe = home.join(format!(".probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
        }
        Err(e) => {
            tracing::warn!(
                event = "sandbox.capability.probe_failed",
                probe = "rootfs_write",
                path = %probe.display(),
                err = %e
            );
            return InstallCapability::VerifyOnly(VerifyOnlyReason::ReadOnlyRoot);
        }
    }

    // 2. Install path: either passwordless sudo, or a user-scope
    //    package manager on PATH.
    //
    //    `apt-get` / `dnf` / `pacman` need root, so they don't count
    //    on their own — only count them if `sudo -n true` succeeds.
    //    `brew` (macOS) and `cargo` (any) install to user-scope
    //    paths and always count.
    if user_scope_pkg_manager_available() {
        return InstallCapability::Full;
    }

    if passwordless_sudo_available() {
        return InstallCapability::Full;
    }

    InstallCapability::VerifyOnly(VerifyOnlyReason::NoInstallPath)
}

fn user_scope_pkg_manager_available() -> bool {
    // Use the `which` crate (already a project dep) — it honors the
    // exec bit on Unix, `PATHEXT` on Windows, and POSIX PATH=. rules.
    ["brew", "cargo", "winget", "scoop"]
        .iter()
        .any(|c| which::which(c).is_ok())
}

fn passwordless_sudo_available() -> bool {
    // `sudo -n true` exits non-zero if a password would be required.
    // Fast (<10ms) and side-effect-free.
    std::process::Command::new("sudo")
        .args(["-n", "true"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Whether the seamless-mode banner has already been emitted in this
/// process. Set by `print_banner_once()`. `AtomicBool` is cheaper
/// than `OnceLock<()>` for a one-shot flag — no allocation, single
/// atomic CAS.
static BANNER_EMITTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Print the one-line seamless-mode banner to stderr.
///
/// Idempotent **per process** (not per `run_setup` call). The
/// process-scoped guarantee is intentional: jarvy is a one-shot CLI,
/// and the banner is a header for the whole invocation. Library
/// embedders that run multiple setups in one process should mirror
/// the message through the `sandbox.detected` tracing event instead
/// (already emitted on first `detect()`).
///
/// Caller must decide whether `--quiet` should suppress the stderr
/// line. The tracing event fires regardless.
pub fn print_banner_once(env_: &SandboxEnvironment) {
    if BANNER_EMITTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let suffix = if env_.forced { " (forced)" } else { "" };
    eprintln!(
        "[jarvy] detected {} — seamless mode{} (override: JARVY_SANDBOX=0)",
        env_.provider, suffix
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every sandbox provider env var. `with_env` clears all of these
    /// before setting the test's target vars so the test runs with a
    /// known-empty baseline regardless of which sandbox the test
    /// runner itself happens to be inside. Serialization across tests
    /// (this module AND `crate::ci::tests`) is enforced by
    /// `#[serial_test::serial(ci_sandbox_env)]` on each test —
    /// shared lock name so the two suites don't race on CI/sandbox
    /// env vars they both touch.
    const SANDBOX_VARS: &[&str] = &[
        "JARVY_SANDBOX",
        "CODESPACES",
        "CODESPACE_NAME",
        "GITPOD_WORKSPACE_ID",
        "REMOTE_CONTAINERS",
        "DEVCONTAINER",
        "REPL_ID",
        "E2B_SANDBOX_ID",
        "MODAL_TASK_ID",
        "DAYTONA_WS_ID",
        "CLAUDECODE",
        "CLAUDE_CODE_ENTRYPOINT",
        "CURSOR_AGENT",
        // Also clear the CI vars so is_seamless() tests are
        // deterministic across runners.
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "JARVY_CI",
        "JARVY_NO_CI",
    ];

    #[allow(unsafe_code)]
    fn with_env<F, R>(vars: &[(&str, &str)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Snapshot + clear every provider var so the test starts
        // from a clean slate.
        // SAFETY: callers are gated by `#[serial(ci_sandbox_env)]`
        // so no other test in this lock group runs concurrently.
        let cleared: Vec<(&str, Option<String>)> = SANDBOX_VARS
            .iter()
            .map(|k| {
                let orig = env::var(k).ok();
                unsafe { env::remove_var(k) };
                (*k, orig)
            })
            .collect();

        // Apply the test's targeted vars.
        let originals: Vec<_> = vars
            .iter()
            .map(|(k, v)| {
                let orig = env::var(k).ok();
                unsafe { env::set_var(k, v) };
                (*k, orig)
            })
            .collect();

        let result = f();

        for (k, orig) in originals {
            match orig {
                Some(v) => unsafe { env::set_var(k, v) },
                None => unsafe { env::remove_var(k) },
            }
        }
        for (k, orig) in cleared {
            match orig {
                Some(v) => unsafe { env::set_var(k, v) },
                None => unsafe { env::remove_var(k) },
            }
        }

        result
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_codespaces() {
        with_env(&[("CODESPACES", "true")], || {
            let env = detect().expect("codespaces should detect");
            assert_eq!(env.provider, SandboxProvider::Codespaces);
            assert!(!env.forced);
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_codespaces_via_name() {
        with_env(&[("CODESPACE_NAME", "test-codespace")], || {
            let env = detect().expect("codespaces should detect via name");
            assert_eq!(env.provider, SandboxProvider::Codespaces);
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_gitpod() {
        with_env(&[("GITPOD_WORKSPACE_ID", "ws-1234")], || {
            assert_eq!(detect().map(|e| e.provider), Some(SandboxProvider::Gitpod));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_devcontainer_via_remote_containers() {
        with_env(&[("REMOTE_CONTAINERS", "true")], || {
            assert_eq!(
                detect().map(|e| e.provider),
                Some(SandboxProvider::Devcontainer)
            );
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_devcontainer_via_devcontainer_env() {
        with_env(&[("DEVCONTAINER", "true")], || {
            assert_eq!(
                detect().map(|e| e.provider),
                Some(SandboxProvider::Devcontainer)
            );
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_replit() {
        with_env(&[("REPL_ID", "abc")], || {
            assert_eq!(detect().map(|e| e.provider), Some(SandboxProvider::Replit));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_e2b() {
        with_env(&[("E2B_SANDBOX_ID", "sb-123")], || {
            assert_eq!(detect().map(|e| e.provider), Some(SandboxProvider::E2b));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_modal() {
        with_env(&[("MODAL_TASK_ID", "task-123")], || {
            assert_eq!(detect().map(|e| e.provider), Some(SandboxProvider::Modal));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_daytona() {
        with_env(&[("DAYTONA_WS_ID", "ws-1")], || {
            assert_eq!(detect().map(|e| e.provider), Some(SandboxProvider::Daytona));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_claude_code_via_claudecode_var() {
        with_env(&[("CLAUDECODE", "1")], || {
            assert_eq!(
                detect().map(|e| e.provider),
                Some(SandboxProvider::ClaudeCode)
            );
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_claude_code_via_entrypoint() {
        with_env(&[("CLAUDE_CODE_ENTRYPOINT", "cli")], || {
            assert_eq!(
                detect().map(|e| e.provider),
                Some(SandboxProvider::ClaudeCode)
            );
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn detects_cursor_agent() {
        with_env(&[("CURSOR_AGENT", "1")], || {
            assert_eq!(detect().map(|e| e.provider), Some(SandboxProvider::Cursor));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn no_detection_in_clean_env() {
        with_env(&[], || {
            // Generic-container fallback may trip if the test runner
            // itself is in a container with /.dockerenv. Accept either
            // None or GenericContainer here.
            match detect() {
                None => {}
                Some(env) => assert_eq!(env.provider, SandboxProvider::GenericContainer),
            }
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn forced_via_jarvy_sandbox_1() {
        with_env(&[("JARVY_SANDBOX", "1")], || {
            let env = detect().expect("forced should detect");
            assert!(env.forced);
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn disabled_via_jarvy_sandbox_0() {
        // Set both an override-off flag and a Codespaces signal;
        // the override should win.
        with_env(&[("JARVY_SANDBOX", "0"), ("CODESPACES", "true")], || {
            assert!(detect().is_none());
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn is_seamless_true_for_sandbox() {
        with_env(&[("CODESPACES", "true")], || {
            assert!(is_seamless());
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn is_seamless_true_for_ci_only() {
        with_env(&[("CI", "true")], || {
            // Sandbox should not detect, but CI should, so seamless
            // is still true.
            assert!(is_seamless());
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn provider_names_are_stable() {
        // Stability matters because users grep telemetry / logs for
        // these strings. Don't change them without a release note.
        assert_eq!(SandboxProvider::Codespaces.name(), "GitHub Codespaces");
        assert_eq!(SandboxProvider::ClaudeCode.name(), "Claude Code");
        assert_eq!(SandboxProvider::GenericContainer.name(), "container");
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn forced_with_no_signals_yields_generic_container() {
        // Pin the precise variant chosen on `JARVY_SANDBOX=1` with no
        // other signal — the spec is "forced generic-container."
        with_env(&[("JARVY_SANDBOX", "1")], || {
            let env = detect().expect("forced should detect");
            assert_eq!(env.provider, SandboxProvider::GenericContainer);
            assert!(env.forced);
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn forced_with_named_signal_keeps_named_provider() {
        // `JARVY_SANDBOX=1` plus a named signal must keep the named
        // provider (Codespaces), not collapse to GenericContainer.
        with_env(&[("JARVY_SANDBOX", "1"), ("CODESPACES", "true")], || {
            let env = detect().expect("forced+named should detect");
            assert_eq!(env.provider, SandboxProvider::Codespaces);
            assert!(env.forced);
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn jarvy_sandbox_0_does_not_disable_seamless_when_ci_true() {
        // `JARVY_SANDBOX=0` only disables sandbox detection — CI
        // detection is independent, so `is_seamless` stays true.
        // Pin this so a future refactor that misroutes CI through
        // the same gate is caught.
        with_env(&[("JARVY_SANDBOX", "0"), ("CI", "true")], || {
            assert!(is_seamless());
            assert!(!is_sandbox());
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn is_seamless_auto_excludes_forced_sandbox() {
        // The forced gate (PRD-053 risk row 1): a hostile env that
        // sets `JARVY_SANDBOX=1` must not silently flip telemetry /
        // update gates.
        with_env(&[("JARVY_SANDBOX", "1")], || {
            assert!(is_seamless()); // sandbox detected
            assert!(!is_seamless_auto()); // but forced, so no auto-disable
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn is_seamless_auto_true_for_auto_detected_sandbox() {
        with_env(&[("CODESPACES", "true")], || {
            assert!(is_seamless_auto());
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn is_seamless_auto_true_for_ci() {
        with_env(&[("CI", "true")], || {
            assert!(is_seamless_auto());
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn generic_container_predicate_requires_both_signals() {
        // The PRD says generic-container fallback gates on
        // `/.dockerenv` AND non-TTY, BOTH required. Pin the truth
        // table so flipping `&&` to `||` fails a test.
        assert!(!is_generic_container(false, false));
        assert!(!is_generic_container(false, true));
        assert!(!is_generic_container(true, true)); // TTY = developer shelled in
        assert!(is_generic_container(true, false));
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn verify_only_reason_display_is_stable() {
        // These strings end up in tracing events / log files; users
        // grep them. Pin the wire format.
        assert_eq!(VerifyOnlyReason::NoJarvyHome.to_string(), "no_jarvy_home");
        assert_eq!(VerifyOnlyReason::ReadOnlyRoot.to_string(), "read_only_root");
        assert_eq!(
            VerifyOnlyReason::NoInstallPath.to_string(),
            "no_install_path"
        );
        assert_eq!(VerifyOnlyReason::Forced.to_string(), "forced");
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn force_verify_only_env_short_circuits_probe() {
        // Make sure `JARVY_FORCE_VERIFY_ONLY=1` yields the `Forced`
        // reason, not a real probe result. Note: the cache is *not*
        // disabled under `cfg(test)` for `install_capability()` —
        // this test must be the first to hit it (cargo test names
        // are unordered but unique). Use a fresh process-equivalent
        // by directly calling the probe rather than the cached
        // wrapper.
        with_env(&[("JARVY_FORCE_VERIFY_ONLY", "1")], || {
            let cap = probe_install_capability();
            assert_eq!(cap, InstallCapability::VerifyOnly(VerifyOnlyReason::Forced));
        });
    }

    #[test]
    #[serial_test::serial(ci_sandbox_env)]
    fn banner_idempotent_per_process() {
        // Cannot easily capture stderr here — assert the `AtomicBool`
        // gate flips exactly once. The print path is one `eprintln!`
        // after the gate, so AtomicBool state IS the contract.
        // Reset first in case another test hit it.
        BANNER_EMITTED.store(false, std::sync::atomic::Ordering::Relaxed);
        let env = SandboxEnvironment {
            provider: SandboxProvider::GenericContainer,
            forced: false,
        };
        // First call: should flip the flag and (per code path) emit.
        assert!(!BANNER_EMITTED.load(std::sync::atomic::Ordering::Relaxed));
        print_banner_once(&env);
        assert!(BANNER_EMITTED.load(std::sync::atomic::Ordering::Relaxed));
        // Second call: must early-return without re-flipping
        // (AtomicBool::swap returns the old value = true).
        print_banner_once(&env);
        assert!(BANNER_EMITTED.load(std::sync::atomic::Ordering::Relaxed));
    }
}
