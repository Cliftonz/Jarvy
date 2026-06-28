//! Git hook framework installation (PRD-048)
//!
//! Installs and manages Git pre-commit hooks driven by `jarvy.toml`'s
//! `[git_hooks]` block. Today only the `pre-commit` framework
//! (<https://pre-commit.com>) is supported; the architecture leaves room
//! for `husky` and `lefthook` handlers behind the same `HookFramework`
//! enum without changing the CLI surface.
//!
//! # Why `[git_hooks]` and not `[hooks]`
//!
//! `[hooks]` is already used by `jarvy setup` for `pre_setup` /
//! `post_install` / `post_setup` shell scripts (PRD-003). Adding a
//! `git_hooks = true` knob into that existing block would entangle two
//! unrelated lifecycles. Using a new top-level `[git_hooks]` keeps
//! their semantics independent and lets users mix-and-match (no setup
//! hooks but yes pre-commit, or vice versa).
//!
//! # Trust boundary
//!
//! Pre-commit configs (`.pre-commit-config.yaml`) reference hook repos
//! by URL + revision. `jarvy hooks install` will fetch and execute
//! arbitrary code from those repos at commit time — same trust model as
//! `pre-commit install` itself. Jarvy does NOT add an additional gate
//! here because (a) the user must already trust the repo they're
//! working in, and (b) pre-commit's own `--hook-impl` sandboxing is
//! upstream's responsibility. Remote configs fetched via
//! `jarvy setup --from <url>` are blocked from auto-installing hooks
//! unless `[git_hooks] allow_remote = true` is set in the SOURCE config
//! (mirrors `[packages] allow_remote`).

pub mod config;
pub mod detection;
pub mod precommit;

use std::path::Path;
use thiserror::Error;

#[allow(unused_imports)] // Public re-export for downstream consumers
pub use config::PreCommitConfig;
pub use config::{GitHooksConfig, HookFramework};
pub use detection::detect_framework;
pub use precommit::PreCommitHandler;

/// Errors produced by hook installation / management.
#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook framework `{0}` is not installed; install it before running `jarvy hooks`")]
    FrameworkNotInstalled(String),

    #[error("hook framework `{0}` is configured but not yet supported by jarvy")]
    UnsupportedFramework(String),

    #[error(
        "remote-fetched config attempted to install git hooks without \
         `[git_hooks] allow_remote = true`; refusing to land arbitrary \
         pre-commit hooks from an untrusted source (PRD-054 trust gate)"
    )]
    RemoteRefused,

    #[error("not inside a git repository (no `.git` directory found)")]
    NotAGitRepo,

    #[error("hook installation failed: {0}")]
    InstallFailed(String),

    #[error("hook update failed: {0}")]
    UpdateFailed(String),

    #[error("hook run failed: {0}")]
    RunFailed(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl HookError {
    /// Stable telemetry discriminant. Mirrors the `kind()` pattern used by
    /// `PackageError` and `AiHookError`.
    pub fn kind(&self) -> &'static str {
        match self {
            HookError::FrameworkNotInstalled(_) => "framework_not_installed",
            HookError::UnsupportedFramework(_) => "unsupported_framework",
            HookError::RemoteRefused => "remote_refused",
            HookError::NotAGitRepo => "not_a_git_repo",
            HookError::InstallFailed(_) => "install_failed",
            HookError::UpdateFailed(_) => "update_failed",
            HookError::RunFailed(_) => "run_failed",
            HookError::Config(_) => "config",
            HookError::Io(_) => "io",
        }
    }
}

/// Refuse install / update / run when the config came from a remote
/// `jarvy setup --from <url>` source and `allow_remote` is not set.
/// Review item 5 (P0) — previously the `allow_remote` field was
/// declared but never read, so a friendly-looking remote config could
/// land arbitrary pre-commit hooks on the consuming machine.
fn enforce_remote_gate(config: &GitHooksConfig) -> Result<(), HookError> {
    if config.origin == crate::ai_hooks::ConfigOrigin::Remote && !config.allow_remote {
        if crate::observability::telemetry_gate::is_enabled() {
            tracing::warn!(
                event = "git_hooks.remote_refused",
                reason = "allow_remote_not_set",
            );
        }
        return Err(HookError::RemoteRefused);
    }
    Ok(())
}

/// Install hooks for the configured framework, auto-detecting if the
/// config doesn't pin one. Returns `Ok(true)` when hooks were installed,
/// `Ok(false)` when nothing was configured / detected. Errors are
/// advisory in the setup flow — callers map to a warning, not a fatal
/// exit.
pub fn install_hooks(config: &GitHooksConfig, project_dir: &Path) -> Result<bool, HookError> {
    if !config.enabled {
        return Ok(false);
    }
    enforce_remote_gate(config)?;
    if !project_dir.join(".git").exists() {
        return Err(HookError::NotAGitRepo);
    }

    let framework = match config.framework.or_else(|| detect_framework(project_dir)) {
        Some(f) => f,
        None => return Ok(false),
    };

    match framework {
        HookFramework::PreCommit => {
            let handler = PreCommitHandler::new(
                config.pre_commit.clone().unwrap_or_default(),
                project_dir.to_path_buf(),
            );
            handler.install()?;
            Ok(true)
        }
        HookFramework::Husky | HookFramework::Lefthook | HookFramework::Native => Err(
            HookError::UnsupportedFramework(framework.as_str().to_string()),
        ),
    }
}

/// Update hooks (currently: pre-commit autoupdate). Behavior parallels
/// `install_hooks` — Ok(true) on update, Ok(false) when nothing to do.
pub fn update_hooks(config: &GitHooksConfig, project_dir: &Path) -> Result<bool, HookError> {
    if !config.enabled {
        return Ok(false);
    }
    enforce_remote_gate(config)?;
    let framework = match config.framework.or_else(|| detect_framework(project_dir)) {
        Some(f) => f,
        None => return Ok(false),
    };
    match framework {
        HookFramework::PreCommit => {
            let handler = PreCommitHandler::new(
                config.pre_commit.clone().unwrap_or_default(),
                project_dir.to_path_buf(),
            );
            handler.update()?;
            Ok(true)
        }
        HookFramework::Husky | HookFramework::Lefthook | HookFramework::Native => Err(
            HookError::UnsupportedFramework(framework.as_str().to_string()),
        ),
    }
}

/// List installed hooks (currently: parse `.pre-commit-config.yaml`).
pub fn list_hooks(config: &GitHooksConfig, project_dir: &Path) -> Result<Vec<HookInfo>, HookError> {
    let framework = match config.framework.or_else(|| detect_framework(project_dir)) {
        Some(f) => f,
        None => return Ok(Vec::new()),
    };
    match framework {
        HookFramework::PreCommit => {
            let handler = PreCommitHandler::new(
                config.pre_commit.clone().unwrap_or_default(),
                project_dir.to_path_buf(),
            );
            handler.list()
        }
        HookFramework::Husky | HookFramework::Lefthook | HookFramework::Native => Err(
            HookError::UnsupportedFramework(framework.as_str().to_string()),
        ),
    }
}

/// Run hooks once. `all_files = true` mirrors `pre-commit run
/// --all-files`. `hook_id = Some("black")` runs a single hook.
pub fn run_hooks(
    config: &GitHooksConfig,
    project_dir: &Path,
    all_files: bool,
    hook_id: Option<&str>,
) -> Result<(), HookError> {
    enforce_remote_gate(config)?;
    let framework = match config.framework.or_else(|| detect_framework(project_dir)) {
        Some(f) => f,
        None => {
            return Err(HookError::Config(
                "no hook framework detected; nothing to run".to_string(),
            ));
        }
    };
    match framework {
        HookFramework::PreCommit => {
            let handler = PreCommitHandler::new(
                config.pre_commit.clone().unwrap_or_default(),
                project_dir.to_path_buf(),
            );
            handler.run(all_files, hook_id)
        }
        HookFramework::Husky | HookFramework::Lefthook | HookFramework::Native => Err(
            HookError::UnsupportedFramework(framework.as_str().to_string()),
        ),
    }
}

/// Hook installation status — what `jarvy hooks status` returns.
#[derive(Debug, Clone)]
pub struct HookStatus {
    pub framework: Option<HookFramework>,
    pub installed: bool,
    pub config_path: Option<String>,
    pub hook_count: usize,
}

/// Probe current status: framework detected? installed in `.git/hooks/`?
pub fn hook_status(config: &GitHooksConfig, project_dir: &Path) -> HookStatus {
    let framework = config.framework.or_else(|| detect_framework(project_dir));
    let installed = project_dir
        .join(".git")
        .join("hooks")
        .join("pre-commit")
        .exists();
    let (config_path, hook_count) = match framework {
        Some(HookFramework::PreCommit) => {
            let path = config
                .pre_commit
                .as_ref()
                .map(|c| c.config.clone())
                .unwrap_or_else(|| ".pre-commit-config.yaml".to_string());
            let count = if project_dir.join(&path).exists() {
                let handler = PreCommitHandler::new(
                    config.pre_commit.clone().unwrap_or_default(),
                    project_dir.to_path_buf(),
                );
                handler.list().map(|h| h.len()).unwrap_or(0)
            } else {
                0
            };
            (Some(path), count)
        }
        _ => (None, 0),
    };
    HookStatus {
        framework,
        installed,
        config_path,
        hook_count,
    }
}

/// A single hook entry surfaced by `jarvy hooks list`.
///
/// `hook_type` is reserved for non-pre-commit frameworks that
/// distinguish hook stages (commit-msg, pre-push, etc.). Today every
/// emitted value is `"pre-commit"` — the field exists so adding husky /
/// lefthook later doesn't require a breaking struct change.
#[derive(Debug, Clone)]
pub struct HookInfo {
    pub id: String,
    pub repo: String,
    pub version: String,
    #[allow(dead_code)] // Reserved for husky/lefthook handlers
    pub hook_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_hooks::ConfigOrigin;
    use tempfile::tempdir;

    /// Review item 5 (P0). Remote-origin config with default
    /// `allow_remote = false` must refuse install / update / run.
    #[test]
    fn install_hooks_refuses_remote_without_allow_remote_opt_in() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".pre-commit-config.yaml"), "repos: []").unwrap();
        let cfg = GitHooksConfig {
            enabled: true,
            framework: Some(HookFramework::PreCommit),
            auto_install: true,
            auto_update: false,
            run_after_install: false,
            allow_remote: false,
            pre_commit: None,
            origin: ConfigOrigin::Remote,
        };
        let err = install_hooks(&cfg, tmp.path()).expect_err("remote must refuse");
        assert!(matches!(err, HookError::RemoteRefused), "got {err:?}");
    }

    #[test]
    fn update_hooks_refuses_remote_without_allow_remote_opt_in() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let cfg = GitHooksConfig {
            enabled: true,
            framework: Some(HookFramework::PreCommit),
            auto_install: true,
            auto_update: false,
            run_after_install: false,
            allow_remote: false,
            pre_commit: None,
            origin: ConfigOrigin::Remote,
        };
        let err = update_hooks(&cfg, tmp.path()).expect_err("remote must refuse");
        assert!(matches!(err, HookError::RemoteRefused));
    }

    #[test]
    fn run_hooks_refuses_remote_without_allow_remote_opt_in() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let cfg = GitHooksConfig {
            enabled: true,
            framework: Some(HookFramework::PreCommit),
            auto_install: true,
            auto_update: false,
            run_after_install: false,
            allow_remote: false,
            pre_commit: None,
            origin: ConfigOrigin::Remote,
        };
        let err = run_hooks(&cfg, tmp.path(), false, None).expect_err("remote must refuse");
        assert!(matches!(err, HookError::RemoteRefused));
    }

    /// Remote-origin config WITH explicit `allow_remote = true` passes
    /// the gate (proceeds to framework detection / handler).
    #[test]
    fn install_hooks_accepts_remote_when_explicitly_opted_in() {
        let tmp = tempdir().unwrap();
        // No .git dir → install_hooks returns NotAGitRepo AFTER the
        // gate check passes. That proves the gate didn't fire.
        let cfg = GitHooksConfig {
            enabled: true,
            framework: Some(HookFramework::PreCommit),
            auto_install: true,
            auto_update: false,
            run_after_install: false,
            allow_remote: true,
            pre_commit: None,
            origin: ConfigOrigin::Remote,
        };
        let err =
            install_hooks(&cfg, tmp.path()).expect_err("no .git → NotAGitRepo, NOT RemoteRefused");
        assert!(
            matches!(err, HookError::NotAGitRepo),
            "expected gate to pass + later check to fail with NotAGitRepo; got {err:?}"
        );
    }

    /// Local-origin config (default) is unchanged — no `allow_remote`
    /// needed.
    #[test]
    fn install_hooks_local_origin_unchanged() {
        let tmp = tempdir().unwrap();
        let cfg = GitHooksConfig {
            enabled: true,
            framework: Some(HookFramework::PreCommit),
            auto_install: true,
            auto_update: false,
            run_after_install: false,
            allow_remote: false,
            pre_commit: None,
            origin: ConfigOrigin::Local, // default
        };
        let err = install_hooks(&cfg, tmp.path()).expect_err("no .git → NotAGitRepo");
        assert!(matches!(err, HookError::NotAGitRepo));
    }
}
