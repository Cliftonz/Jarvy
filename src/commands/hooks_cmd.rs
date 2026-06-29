//! `jarvy hooks <action>` command handler (PRD-048).
//!
//! Thin glue: loads the `[git_hooks]` block from `jarvy.toml`, dispatches
//! to `crate::git_hooks::{install,update,run,list,hook_status}`, formats
//! the result for stdout, returns an exit code.

use crate::cli::HooksAction;
use crate::config::Config;
use crate::git_hooks::{self, GitHooksConfig};
use crate::progress::Progress;
use std::path::Path;

pub fn run_hooks(action: &HooksAction, file: &str) -> i32 {
    let config = Config::new(file);
    let git_hooks_config = config.git_hooks.clone().unwrap_or_default();
    let project_dir = Path::new(file)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    match action {
        HooksAction::Install {} => install_action(&git_hooks_config, &project_dir),
        HooksAction::Update {} => update_action(&git_hooks_config, &project_dir),
        HooksAction::Status {} => status_action(&git_hooks_config, &project_dir),
        HooksAction::List {} => list_action(&git_hooks_config, &project_dir),
        HooksAction::Run { all_files, hook } => {
            run_action(&git_hooks_config, &project_dir, *all_files, hook.as_deref())
        }
        HooksAction::Uninstall {} => uninstall_action(&project_dir),
    }
}

fn install_action(config: &GitHooksConfig, project_dir: &Path) -> i32 {
    let progress = Progress::start();
    let spinner = progress.add("[hooks]", "Installing git hooks...");
    match git_hooks::install_hooks(config, project_dir) {
        Ok(true) => {
            spinner.finish_ok("git hooks installed");
            0
        }
        Ok(false) => {
            spinner.finish_skipped("nothing configured");
            println!(
                "  No hook framework detected and none pinned in [git_hooks]. \
                 Add a `.pre-commit-config.yaml` or set `framework = \"pre-commit\"`."
            );
            0
        }
        Err(e) => {
            spinner.finish_failed(format!("{e}"));
            crate::error_codes::HOOK_FAILED
        }
    }
}

fn update_action(config: &GitHooksConfig, project_dir: &Path) -> i32 {
    let progress = Progress::start();
    let spinner = progress.add("[hooks]", "Updating git hooks...");
    match git_hooks::update_hooks(config, project_dir) {
        Ok(true) => {
            spinner.finish_ok("hooks updated");
            0
        }
        Ok(false) => {
            spinner.finish_skipped("nothing configured");
            0
        }
        Err(e) => {
            spinner.finish_failed(format!("{e}"));
            crate::error_codes::HOOK_FAILED
        }
    }
}

fn status_action(config: &GitHooksConfig, project_dir: &Path) -> i32 {
    let status = git_hooks::hook_status(config, project_dir);
    println!("Git Hooks Status");
    println!("================");
    match status.framework {
        Some(f) => println!("Framework:    {}", f.as_str()),
        None => println!("Framework:    (none detected)"),
    }
    println!(
        "Installed:    {}",
        if status.installed { "yes" } else { "no" }
    );
    if let Some(path) = status.config_path {
        println!("Config:       {path}");
    }
    println!("Hook count:   {}", status.hook_count);
    0
}

fn list_action(config: &GitHooksConfig, project_dir: &Path) -> i32 {
    match git_hooks::list_hooks(config, project_dir) {
        Ok(hooks) if hooks.is_empty() => {
            println!("No hooks configured.");
            0
        }
        Ok(hooks) => {
            println!("Configured hooks ({}):", hooks.len());
            // Group by repo for readability.
            let mut current_repo = String::new();
            for h in &hooks {
                if h.repo != current_repo {
                    println!();
                    if h.repo == "local" {
                        println!("  local");
                    } else {
                        println!("  {} ({})", h.repo, h.version);
                    }
                    current_repo.clone_from(&h.repo);
                }
                println!("    {}", h.id);
            }
            0
        }
        Err(e) => {
            eprintln!("Failed to list hooks: {e}");
            crate::error_codes::CONFIG_ERROR
        }
    }
}

fn run_action(
    config: &GitHooksConfig,
    project_dir: &Path,
    all_files: bool,
    hook: Option<&str>,
) -> i32 {
    // `pre-commit run` streams its own output. Skip the progress
    // spinner — it would clash with the subprocess's stdout.
    match git_hooks::run_hooks(config, project_dir, all_files, hook) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Hook run failed: {e}");
            crate::error_codes::HOOK_FAILED
        }
    }
}

fn uninstall_action(project_dir: &Path) -> i32 {
    // `pre-commit uninstall` is the only uninstall path supported today.
    // Bypass the handler abstraction — `update_hooks` etc. require a
    // framework decision but uninstall doesn't need one.
    use std::process::Command;
    let status = Command::new("pre-commit")
        .arg("uninstall")
        .current_dir(project_dir)
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("pre-commit hooks uninstalled");
            0
        }
        Ok(s) => {
            eprintln!(
                "pre-commit uninstall exited with {}",
                s.code().unwrap_or(-1)
            );
            crate::error_codes::HOOK_FAILED
        }
        Err(e) => {
            eprintln!("Failed to invoke `pre-commit uninstall`: {e}");
            crate::error_codes::HOOK_FAILED
        }
    }
}

#[cfg(test)]
mod tests {
    //! Exit-code contract tests for `hooks_cmd` action handlers
    //! (QA P1, review item 25). These pin the install/update/list/status
    //! mapping so a refactor of `git_hooks::*` can't silently flip a
    //! success path into a HOOK_FAILED exit (or vice versa) without a
    //! test catching it.
    use super::*;
    use crate::ai_hooks::ConfigOrigin;
    use crate::git_hooks::{GitHooksConfig, HookFramework};
    use tempfile::tempdir;

    fn cfg_local_pre_commit() -> GitHooksConfig {
        GitHooksConfig {
            enabled: true,
            framework: Some(HookFramework::PreCommit),
            auto_install: true,
            auto_update: false,
            run_after_install: false,
            allow_remote: false,
            pre_commit: None,
            native: None,
            origin: ConfigOrigin::Local,
        }
    }

    fn cfg_disabled() -> GitHooksConfig {
        let mut c = cfg_local_pre_commit();
        c.enabled = false;
        c
    }

    fn cfg_remote_without_opt_in() -> GitHooksConfig {
        let mut c = cfg_local_pre_commit();
        c.origin = ConfigOrigin::Remote;
        c
    }

    /// `install_action` returns 0 (Ok) when hooks are disabled —
    /// `install_hooks` returns `Ok(false)` and the action treats that
    /// as a non-error skip.
    #[test]
    fn install_action_returns_zero_when_disabled() {
        let tmp = tempdir().unwrap();
        let exit = install_action(&cfg_disabled(), tmp.path());
        assert_eq!(exit, 0);
    }

    /// `install_action` returns HOOK_FAILED when the project isn't a
    /// git repo (the underlying handler returns `Err(NotAGitRepo)`).
    /// Pins the "any error → HOOK_FAILED" contract.
    #[test]
    fn install_action_returns_hook_failed_when_not_a_git_repo() {
        let tmp = tempdir().unwrap();
        // No .git directory created.
        let exit = install_action(&cfg_local_pre_commit(), tmp.path());
        assert_eq!(exit, crate::error_codes::HOOK_FAILED);
    }

    /// Remote-origin config without `allow_remote` opt-in must return
    /// HOOK_FAILED — the trust gate fires inside `install_hooks` and
    /// the action surfaces it as a failure exit. Review item 5 (P0)
    /// already covered the refusal; this pins the CLI-level exit code.
    #[test]
    fn install_action_returns_hook_failed_for_remote_without_allow_remote() {
        let tmp = tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let exit = install_action(&cfg_remote_without_opt_in(), tmp.path());
        assert_eq!(exit, crate::error_codes::HOOK_FAILED);
    }

    /// `update_action` returns 0 when disabled.
    #[test]
    fn update_action_returns_zero_when_disabled() {
        let tmp = tempdir().unwrap();
        let exit = update_action(&cfg_disabled(), tmp.path());
        assert_eq!(exit, 0);
    }

    /// Remote origin must refuse update too.
    #[test]
    fn update_action_returns_hook_failed_for_remote_without_allow_remote() {
        let tmp = tempdir().unwrap();
        let exit = update_action(&cfg_remote_without_opt_in(), tmp.path());
        assert_eq!(exit, crate::error_codes::HOOK_FAILED);
    }

    /// `status_action` is a pure probe — always 0, even when nothing
    /// is detected or installed. Mirrors `jarvy hooks status` against
    /// a fresh non-git directory.
    #[test]
    fn status_action_returns_zero_on_empty_dir() {
        let tmp = tempdir().unwrap();
        let exit = status_action(&cfg_local_pre_commit(), tmp.path());
        assert_eq!(exit, 0);
    }

    /// `list_action` with no detected framework returns 0 (treated as
    /// "nothing to list" by the handler).
    #[test]
    fn list_action_returns_zero_when_no_framework_detected() {
        let tmp = tempdir().unwrap();
        let mut cfg = cfg_local_pre_commit();
        cfg.framework = None;
        let exit = list_action(&cfg, tmp.path());
        assert_eq!(exit, 0);
    }

    /// `list_action` with a pre-commit framework set but no config
    /// file present returns 0 (the handler treats missing config as
    /// "no hooks configured", not an error).
    #[test]
    fn list_action_returns_zero_when_config_file_missing() {
        let tmp = tempdir().unwrap();
        let exit = list_action(&cfg_local_pre_commit(), tmp.path());
        assert_eq!(exit, 0);
    }

    /// `run_action` returns HOOK_FAILED when the trust gate refuses.
    #[test]
    fn run_action_returns_hook_failed_for_remote_without_allow_remote() {
        let tmp = tempdir().unwrap();
        let exit = run_action(&cfg_remote_without_opt_in(), tmp.path(), false, None);
        assert_eq!(exit, crate::error_codes::HOOK_FAILED);
    }
}
