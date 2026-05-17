//! End-to-end test that exercises the locally built `jarvy` binary
//! across every Linux distro Jarvy supports.
//!
//! Bind-mounts the locally built `jarvy` (read-only) into a fresh
//! container per distro and runs two smoke commands:
//!   1. `jarvy --version` — must match `^jarvy \d+(\.\d+)+$` on stdout
//!      with stderr empty. The version digits are not compared against
//!      `CARGO_PKG_VERSION` because `src/cli/args.rs` hardcodes the
//!      clap display string independently.
//!   2. `jarvy tools --default-hooks` — touches the OS detector and
//!      tool registry walk so a binary broken outside clap-argv parsing
//!      fails loudly here, not silently in the user's first real run.
//!
//! Cross-arch + skip conditions follow the same harness as
//! `tests/sandbox_integration.rs` — see that file for the longer
//! explanation.
//!
//! ## Opt-in & loud-fail
//!
//! Disabled by default (network-free `cargo test` stays fast). Enable
//! with `JARVY_E2E_INSTALL=1`. On Linux CI (`CI=true`) with a
//! reachable docker daemon, **forgetting** the gate panics — a silently
//! green matrix is a regression risk.
//!
//! ## Env knobs
//!
//! | Var                  | Default  | Effect                                      |
//! |----------------------|----------|---------------------------------------------|
//! | `JARVY_E2E_INSTALL`  | unset    | Master switch (opt-in)                      |
//! | `JARVY_TEST_BIN`     | cargo-built | Override mounted binary (cross-builds)   |
//! | `JARVY_BIN_LIBC`     | `glibc`  | Selects Alpine green/red path (`glibc`/`musl`) |
//! | `JARVY_E2E_THREADS`  | 4        | Makefile-only knob; see Makefile            |
//!
//! ## Distro matrix
//!
//! | Distro                  | PM      | libc  |
//! |-------------------------|---------|-------|
//! | `ubuntu:22.04`          | apt     | glibc |
//! | `ubuntu:24.04`          | apt     | glibc |
//! | `debian:bookworm-slim`  | apt     | glibc |
//! | `fedora:40`             | dnf     | glibc |
//! | `rockylinux:9`          | dnf     | glibc |
//! | `amazonlinux:2`         | yum     | musl* |
//! | `archlinux:latest`      | pacman  | glibc |
//! | `opensuse/leap:15.6`    | zypper  | glibc |
//! | `alpine:3.20`           | apk     | musl  |
//!
//! \* Amazon Linux 2 ships glibc 2.26; cross-built glibc binaries
//! typically require ≥ 2.27. Tagged musl so the test runs only when a
//! musl-static jarvy is mounted. See `install_pipeline_amazonlinux_2`.
//!
//! ## Image digests
//!
//! Every base image is pinned by manifest-list `@sha256:` digest so
//! a registry tag re-push (legit refresh or namespace takeover) cannot
//! silently change what CI executes. To bump, run
//! `docker buildx imagetools inspect <ref> --format '{{.Manifest.Digest}}'`
//! and paste the result below.

mod common;

use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use common::{
    CONTAINER_HOME, CONTAINER_JARVY_BIN, CONTAINER_JARVY_HOME, CONTAINER_LIFETIME_SECS,
    docker_available, host_jarvy_path, is_linux_elf, scrub_for_panic, short_sha256,
};
use testcontainers::core::{AccessMode, CmdWaitFor, ExecCommand, Mount};
use testcontainers::runners::SyncRunner;
use testcontainers::{GenericImage, ImageExt};

// Pinned manifest-list digests. Pinned 2026-05-14. See bump procedure
// in the module header.
const UBUNTU_22_04_DIGEST: &str =
    "sha256:962f6cadeae0ea6284001009daa4cc9a8c37e75d1f5191cf0eb83fe565b63dd7";
const UBUNTU_24_04_DIGEST: &str =
    "sha256:c4a8d5503dfb2a3eb8ab5f807da5bc69a85730fb49b5cfca2330194ebcc41c7b";
const DEBIAN_BOOKWORM_SLIM_DIGEST: &str =
    "sha256:67b30a61dc87758f0caf819646104f29ecbda97d920aaf5edc834128ac8493d3";
const FEDORA_40_DIGEST: &str =
    "sha256:3c86d25fef9d2001712bc3d9b091fc40cf04be4767e48f1aa3b785bf58d300ed";
const ROCKY_9_DIGEST: &str =
    "sha256:d7be1c094cc5845ee815d4632fe377514ee6ebcf8efaed6892889657e5ddaaa6";
const AMAZONLINUX_2_DIGEST: &str =
    "sha256:74e5c80ad36e6ef0f6fd4a55bb3cc969c05dec6b9dc27fdfa68c8e77264901f9";
const ARCHLINUX_LATEST_DIGEST: &str =
    "sha256:ceac417c19645d21630c120fa123942aa1fc5988faab14e67222013cb11f31bb";
const ALPINE_3_20_DIGEST: &str =
    "sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc";
const OPENSUSE_LEAP_15_6_DIGEST: &str =
    "sha256:79be7751205ea84559990fb76b1bec71e38d6fad41c70a4f6c921b803b58f421";

/// The libc the locally built jarvy was compiled against. Defaults to
/// `glibc` (matching the Makefile's `aarch64-unknown-linux-gnu` cross
/// target). Override with `JARVY_BIN_LIBC=musl` after a musl
/// cross-build to flip the Alpine green/red split.
fn host_jarvy_libc() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED
        .get_or_init(|| std::env::var("JARVY_BIN_LIBC").unwrap_or_else(|_| "glibc".into()))
        .as_str()
}

/// Memoized skip-reason. Computed once; every test calls cheaply.
/// **Intentionally not extracted to `tests/common/mod.rs`** — the
/// opt-in env var, CI loud-fail rule, and message tail differ from
/// `sandbox_integration.rs::skip_reason`. Merging would force a
/// boolean-laden config arg that gets worse with every new test
/// family. Counterweight per PRD-054 review F2.
fn skip_reason() -> Option<&'static str> {
    static SKIP: OnceLock<Option<String>> = OnceLock::new();
    SKIP.get_or_init(compute_skip_reason).as_deref()
}

fn compute_skip_reason() -> Option<String> {
    let opted_in = std::env::var("JARVY_E2E_INSTALL").ok().as_deref() == Some("1");
    let on_ci = std::env::var("CI").ok().as_deref() == Some("true");
    let is_linux = cfg!(target_os = "linux");

    if !opted_in {
        // Loud-fail on Linux CI: a silent green matrix is a regression
        // risk worse than the test not running.
        if on_ci && is_linux && docker_available() {
            panic!(
                "JARVY_E2E_INSTALL=1 is required on Linux CI with docker. \
                 A silently-skipped install-pipeline matrix is a regression risk."
            );
        }
        return Some("JARVY_E2E_INSTALL=1 not set (opt-in)".into());
    }
    if !docker_available() {
        return Some("docker daemon not reachable".into());
    }
    let bin = host_jarvy_path();
    if !bin.exists() {
        return Some(format!("jarvy binary not found at {}", bin.display()));
    }
    if !is_linux_elf(&bin) {
        return Some(format!(
            "binary at {} is not a Linux ELF — set JARVY_TEST_BIN to a \
             cross-built linux jarvy (try `make test-install-pipeline`)",
            bin.display()
        ));
    }
    if let Err(why) = validate_host_bin(&bin) {
        return Some(why);
    }
    None
}

/// Reject `JARVY_TEST_BIN` paths that aren't safe to bind-mount: must
/// canonicalize, must not be group/world-writable. Closes the TOCTOU +
/// foot-gun gap surfaced by PRD-054 review F12.
fn validate_host_bin(bin: &Path) -> Result<(), String> {
    let canonical = std::fs::canonicalize(bin)
        .map_err(|e| format!("canonicalize {} failed: {e}", bin.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&canonical)
            .map_err(|e| format!("stat {} failed: {e}", canonical.display()))?
            .permissions()
            .mode();
        if mode & 0o022 != 0 {
            return Err(format!(
                "{} is group/world-writable (mode {:o}) — refusing to mount",
                canonical.display(),
                mode
            ));
        }
    }
    Ok(())
}

/// Which libc family a distro image expects from the mounted binary.
#[derive(Clone, Copy)]
enum LibcKind {
    Glibc,
    Musl,
}

impl LibcKind {
    fn name(self) -> &'static str {
        match self {
            Self::Glibc => "glibc",
            Self::Musl => "musl",
        }
    }
}

/// Spec for one container test. Keeps the per-distro call sites short.
struct SmokeSpec {
    /// Display name in panic messages and log breadcrumbs.
    label: &'static str,
    /// Docker Hub image (e.g. `"ubuntu"`).
    image: &'static str,
    /// Tag the digest pins (e.g. `"22.04"`).
    tag: &'static str,
    /// Manifest-list digest, `sha256:...`.
    digest: &'static str,
    /// libc this image's loader supports.
    libc: LibcKind,
}

/// Run `jarvy --version` + `jarvy tools --default-hooks` inside the
/// spec's container. Panics with a context-rich message on any
/// failure.
fn run_smoke(spec: &SmokeSpec) {
    if let Some(why) = skip_reason() {
        println!("SKIP[{}]: {why}", spec.label);
        return;
    }

    // Libc mismatch is not a skip — the dedicated alpine_glibc test
    // owns the failure case. Here we want a clean success path only.
    if host_jarvy_libc() != spec.libc.name() {
        println!(
            "SKIP[{}]: host jarvy is {} but image needs {} (see install_pipeline_alpine_3_20_glibc_expected_loader_failure)",
            spec.label,
            host_jarvy_libc(),
            spec.libc.name(),
        );
        return;
    }

    match exec_smoke(spec) {
        SmokeResult::Pass => {}
        SmokeResult::Skip(reason) => {
            println!("SKIP[{}]: {reason}", spec.label);
        }
        SmokeResult::Fail(msg) => panic!("{msg}"),
    }
}

/// Outcome of one container smoke. `Skip` covers infrastructure
/// issues (Docker Hub rate-limit after retry, manifest-missing for
/// host arch) that should not red-flag CI — those are environment
/// problems, not jarvy regressions.
enum SmokeResult {
    Pass,
    Skip(String),
    Fail(String),
}

/// Internal — does the work of `run_smoke` but returns a `SmokeResult`
/// so we can also call it from the alpine_glibc expected-failure test
/// without panicking, and so infra flakes become Skips not Fails.
fn exec_smoke(spec: &SmokeSpec) -> SmokeResult {
    let started = Instant::now();
    let jarvy = host_jarvy_path();
    let bin_sha = short_sha256(&jarvy);
    let host_arch = std::env::consts::ARCH;
    let pinned_tag = format!("{}@{}", spec.tag, spec.digest);

    eprintln!(
        "[{}] starting (image={}:{} digest={} host_arch={} bin={} bin_sha={})",
        spec.label,
        spec.image,
        spec.tag,
        spec.digest,
        host_arch,
        jarvy.display(),
        bin_sha
    );

    // ContainerRequest is not Clone; build a fresh one per attempt
    // via this closure so we can retry on transient Docker Hub errors.
    //
    // No `with_wait_for` — `CmdWaitFor::exit()` on the subsequent exec
    // already blocks on completion; a wall-clock sleep would burn time
    // for no signal (PRD-054 perf review F1).
    let build_request = || {
        GenericImage::new(spec.image, pinned_tag.as_str())
            .with_cmd(vec!["sleep", CONTAINER_LIFETIME_SECS])
            .with_mount(
                Mount::bind_mount(jarvy.to_string_lossy().into_owned(), CONTAINER_JARVY_BIN)
                    .with_access_mode(AccessMode::ReadOnly),
            )
            .with_env_var("JARVY_HOME", CONTAINER_JARVY_HOME)
            .with_env_var("HOME", CONTAINER_HOME)
            .with_env_var("JARVY_TELEMETRY", "0")
            // Defense-in-depth: smoke test does not need root or
            // default caps. RO rootfs would also be ideal but some
            // distros' init scripts write to /var even on a `sleep`
            // cmd; leave that off.
            .with_cap_drop("ALL")
            .with_security_opt("no-new-privileges:true")
    };

    // One-shot retry on transient Docker Hub blips (rate limit, 5xx).
    // Pre-pull catches most of these, but a fresh manifest fetch can
    // still race the limit when 8 distros pull in parallel. If retry
    // still hits a transient error, return Skip (infra problem, not a
    // jarvy regression) so CI doesn't red-flag an unauthenticated
    // Docker Hub rate cap.
    let container = match build_request().start() {
        Ok(c) => c,
        Err(e) if is_transient_pull_error(&e.to_string()) => {
            eprintln!("[{}] transient pull error, retrying in 5s: {e}", spec.label);
            std::thread::sleep(std::time::Duration::from_secs(5));
            match build_request().start() {
                Ok(c) => c,
                Err(e2) if is_transient_pull_error(&e2.to_string()) => {
                    return SmokeResult::Skip(format!(
                        "transient docker-hub error after retry ({e2}) — likely unauthenticated pull rate limit"
                    ));
                }
                Err(e2) => {
                    return SmokeResult::Fail(format!(
                        "[{}] start failed after retry (image={}:{} digest={}): {e2}",
                        spec.label, spec.image, spec.tag, spec.digest
                    ));
                }
            }
        }
        Err(e) => {
            return SmokeResult::Fail(format!(
                "[{}] start failed (image={}:{} digest={}): {e}",
                spec.label, spec.image, spec.tag, spec.digest
            ));
        }
    };
    let container_id = container.id().to_string();

    eprintln!(
        "[{}] container_id={} ready (t={}ms)",
        spec.label,
        container_id,
        started.elapsed().as_millis()
    );

    // Smoke 1: --version. Asserts the output shape `jarvy X.Y[.Z...]`
    // and that stderr stays empty. Catches stubs that just print
    // "jarvy" (no digits) or route output to stderr. The exact version
    // string isn't compared against `CARGO_PKG_VERSION` because
    // `src/cli/args.rs` hardcodes the clap version display string
    // independently of the package version.
    let version_out = match exec_capture(&container, &["jarvy", "--version"]) {
        Ok(o) => o,
        Err(e) => {
            return SmokeResult::Fail(format!(
                "[{}] --version exec failed (container={}): {e}",
                spec.label, container_id
            ));
        }
    };
    if version_out.exit_code != 0 {
        return SmokeResult::Fail(format!(
            "[{}] `jarvy --version` exited {} on {}:{} (container={}, bin_sha={})\nstdout:\n{}\nstderr:\n{}",
            spec.label,
            version_out.exit_code,
            spec.image,
            spec.tag,
            container_id,
            bin_sha,
            scrub_for_panic(&version_out.stdout),
            scrub_for_panic(&version_out.stderr),
        ));
    }
    if !version_pattern().is_match(version_out.stdout.trim()) {
        return SmokeResult::Fail(format!(
            "[{}] stdout does not match `^jarvy \\d+(\\.\\d+)+$`; got `{}` (container={}, bin_sha={})\nstderr:\n{}",
            spec.label,
            version_out.stdout.trim(),
            container_id,
            bin_sha,
            scrub_for_panic(&version_out.stderr),
        ));
    }
    if !version_out.stderr.trim().is_empty() {
        return SmokeResult::Fail(format!(
            "[{}] `jarvy --version` wrote to stderr on {}:{} (container={}, bin_sha={})\nstderr:\n{}",
            spec.label,
            spec.image,
            spec.tag,
            container_id,
            bin_sha,
            scrub_for_panic(&version_out.stderr),
        ));
    }

    // Smoke 2: stack-touching subcommand. `--default-hooks` walks the
    // tool registry, exercising the inventory init + OS detection
    // path. A binary broken in tokio/tracing/registry init fails here.
    let hooks_out = match exec_capture(&container, &["jarvy", "tools", "--default-hooks"]) {
        Ok(o) => o,
        Err(e) => {
            return SmokeResult::Fail(format!(
                "[{}] tools --default-hooks exec failed (container={}): {e}",
                spec.label, container_id
            ));
        }
    };
    if hooks_out.exit_code != 0 {
        return SmokeResult::Fail(format!(
            "[{}] `jarvy tools --default-hooks` exited {} on {}:{} (container={}, bin_sha={})\nstdout:\n{}\nstderr:\n{}",
            spec.label,
            hooks_out.exit_code,
            spec.image,
            spec.tag,
            container_id,
            bin_sha,
            scrub_for_panic(&hooks_out.stdout),
            scrub_for_panic(&hooks_out.stderr),
        ));
    }

    eprintln!(
        "[{}] OK (total={}ms)",
        spec.label,
        started.elapsed().as_millis()
    );
    SmokeResult::Pass
}

/// Match docker pull errors worth retrying once. Covers Docker Hub
/// rate limit (429), 5xx, and the generic "failed to pull" wrapping
/// testcontainers applies. Returns false for permanent failures like
/// "no matching manifest" (wrong arch) which a retry can't fix.
fn is_transient_pull_error(msg: &str) -> bool {
    let lc = msg.to_lowercase();
    (lc.contains("pull rate limit")
        || lc.contains("too many requests")
        || lc.contains("status code 429")
        || lc.contains("status code 500")
        || lc.contains("status code 502")
        || lc.contains("status code 503")
        || lc.contains("status code 504"))
        && !lc.contains("no matching manifest")
}

/// `^jarvy X.Y[.Z...]$` — matches the clap-rendered version line
/// without coupling the assertion to a specific version number.
/// Compiled once, shared across all 9 tests.
fn version_pattern() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^jarvy \d+(?:\.\d+)+$").expect("version regex compiles"))
}

/// Captured stdio + exit code from one `exec` inside a container.
struct ExecOut {
    exit_code: i64,
    stdout: String,
    stderr: String,
}

fn exec_capture(
    container: &testcontainers::Container<GenericImage>,
    cmd: &[&str],
) -> Result<ExecOut, String> {
    let exec_cmd = ExecCommand::new(cmd.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .with_cmd_ready_condition(CmdWaitFor::exit());
    let mut run = container
        .exec(exec_cmd)
        .map_err(|e| format!("exec({:?}): {e}", cmd))?;
    let stdout = String::from_utf8_lossy(&run.stdout_to_vec().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&run.stderr_to_vec().unwrap_or_default()).into_owned();
    let exit_code = run
        .exit_code()
        .map_err(|e| format!("exit_code({:?}): {e}", cmd))?
        .ok_or_else(|| format!("no exit code for {:?}", cmd))?;
    Ok(ExecOut {
        exit_code,
        stdout,
        stderr,
    })
}

// ---------------------------------------------------------------------
// Per-distro tests. One `#[test]` per supported package-manager family.
// ---------------------------------------------------------------------

#[test]
fn install_pipeline_ubuntu_22_04() {
    run_smoke(&SmokeSpec {
        label: "ubuntu_22_04",
        image: "ubuntu",
        tag: "22.04",
        digest: UBUNTU_22_04_DIGEST,
        libc: LibcKind::Glibc,
    });
}

#[test]
fn install_pipeline_ubuntu_24_04() {
    run_smoke(&SmokeSpec {
        label: "ubuntu_24_04",
        image: "ubuntu",
        tag: "24.04",
        digest: UBUNTU_24_04_DIGEST,
        libc: LibcKind::Glibc,
    });
}

#[test]
fn install_pipeline_debian_bookworm() {
    run_smoke(&SmokeSpec {
        label: "debian_bookworm",
        image: "debian",
        tag: "bookworm-slim",
        digest: DEBIAN_BOOKWORM_SLIM_DIGEST,
        libc: LibcKind::Glibc,
    });
}

#[test]
fn install_pipeline_fedora_40() {
    run_smoke(&SmokeSpec {
        label: "fedora_40",
        image: "fedora",
        tag: "40",
        digest: FEDORA_40_DIGEST,
        libc: LibcKind::Glibc,
    });
}

#[test]
fn install_pipeline_rocky_9() {
    run_smoke(&SmokeSpec {
        label: "rocky_9",
        image: "rockylinux",
        tag: "9",
        digest: ROCKY_9_DIGEST,
        libc: LibcKind::Glibc,
    });
}

/// Covers the `yum` package-manager branch in `src/tools/common.rs`
/// (Amazon Linux 2 ships yum-primary; RHEL 7 et al. did the same).
///
/// Tagged `LibcKind::Musl` — Amazon Linux 2 ships glibc 2.26 which is
/// older than the glibc most rust cross-builds target, so a glibc
/// jarvy fails here with `GLIBC_2.x not found`. A musl-static jarvy
/// runs cleanly. This test runs only when `JARVY_BIN_LIBC=musl`.
#[test]
fn install_pipeline_amazonlinux_2() {
    run_smoke(&SmokeSpec {
        label: "amazonlinux_2",
        image: "amazonlinux",
        tag: "2",
        digest: AMAZONLINUX_2_DIGEST,
        libc: LibcKind::Musl,
    });
}

#[test]
fn install_pipeline_archlinux() {
    run_smoke(&SmokeSpec {
        label: "archlinux",
        image: "archlinux",
        tag: "latest",
        digest: ARCHLINUX_LATEST_DIGEST,
        libc: LibcKind::Glibc,
    });
}

#[test]
fn install_pipeline_opensuse_leap_15_6() {
    run_smoke(&SmokeSpec {
        label: "opensuse_leap_15_6",
        image: "opensuse/leap",
        tag: "15.6",
        digest: OPENSUSE_LEAP_15_6_DIGEST,
        libc: LibcKind::Glibc,
    });
}

/// Alpine green path — requires a musl-built jarvy. Skipped when
/// `JARVY_BIN_LIBC != "musl"`.
#[test]
fn install_pipeline_alpine_3_20_musl() {
    run_smoke(&SmokeSpec {
        label: "alpine_3_20_musl",
        image: "alpine",
        tag: "3.20",
        digest: ALPINE_3_20_DIGEST,
        libc: LibcKind::Musl,
    });
}

/// Alpine expected-failure path — proves that mounting a glibc binary
/// on a musl distro produces the classic "not found" loader error.
/// Encodes the regression guard the prior doc-comment only described.
/// Skipped when the host binary is musl (the green-path test owns
/// that case).
#[test]
fn install_pipeline_alpine_3_20_glibc_expected_loader_failure() {
    if let Some(why) = skip_reason() {
        println!("SKIP[alpine_3_20_glibc]: {why}");
        return;
    }
    if host_jarvy_libc() != "glibc" {
        println!("SKIP[alpine_3_20_glibc]: host jarvy is musl; green-path test owns this case");
        return;
    }

    let spec = SmokeSpec {
        label: "alpine_3_20_glibc",
        image: "alpine",
        tag: "3.20",
        digest: ALPINE_3_20_DIGEST,
        libc: LibcKind::Glibc, // pretend match so exec_smoke runs
    };

    let err = match exec_smoke(&spec) {
        SmokeResult::Pass => panic!(
            "expected glibc jarvy on alpine to fail at the dynamic loader, but exec succeeded — \
             did the release pipeline start shipping a musl-static or universal binary? \
             Update this test if so."
        ),
        SmokeResult::Skip(reason) => {
            println!("SKIP[alpine_3_20_glibc]: {reason}");
            return;
        }
        SmokeResult::Fail(msg) => msg,
    };
    // Loader-mismatch on alpine surfaces in two flavors depending on
    // docker version: either the dynamic loader is missing (exit 127,
    // "not found") or docker exec itself fails to start the binary
    // (exit 255, "no such file or directory"). Either is acceptable
    // proof that the glibc binary cannot run.
    let err_lc = err.to_lowercase();
    assert!(
        err_lc.contains("not found")
            || err_lc.contains("no such file")
            || err_lc.contains("exec format error")
            || err_lc.contains("exited 127")
            || err_lc.contains("exited 255"),
        "expected loader-mismatch signature in the panic, got:\n{err}"
    );
}
