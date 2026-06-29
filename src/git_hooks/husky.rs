//! Husky framework handler.
//!
//! [Husky](https://typicode.github.io/husky/) is the JS-ecosystem hook
//! manager. It shells out to `npx husky install` (v9+) /
//! `npx husky-init` (v8 legacy) to write `.husky/_/` into the repo,
//! then per-hook scripts live in `.husky/<hook-name>` and are executed
//! directly by git.
//!
//! Jarvy's husky handler:
//!
//! 1. Detects husky's presence via `.husky/` directory OR the
//!    `package.json` `prepare = "husky"` script convention.
//! 2. `install`: runs `npx --yes husky install` from the project root
//!    so a fresh clone gets `.husky/_/` populated and git's
//!    `core.hooksPath` redirected. Idempotent — re-running is safe.
//! 3. `update`: `npm install --save-dev husky@latest` then re-installs.
//! 4. `run`: husky doesn't have a "run all hooks" CLI; we read every
//!    executable file under `.husky/` (excluding `_/`) and run it
//!    explicitly.
//! 5. `list`: returns one `HookInfo` per `.husky/<hook-name>` file.
//!
//! Requires `npx` on PATH — surfaced as `FrameworkNotInstalled` if
//! missing.

use super::{HookError, HookInfo};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct HuskyHandler {
    project_dir: PathBuf,
}

impl HuskyHandler {
    pub fn new(project_dir: PathBuf) -> Self {
        Self { project_dir }
    }

    pub fn install(&self) -> Result<(), HookError> {
        if !npx_available() {
            return Err(HookError::FrameworkNotInstalled("npx".to_string()));
        }

        // Ensure `husky` is on the project's devDependencies. We use
        // `npm install --save-dev husky` so it lands in package.json.
        let pkg = self.project_dir.join("package.json");
        if !pkg.exists() {
            return Err(HookError::Config(format!(
                "husky requires package.json; none found at {}",
                pkg.display()
            )));
        }

        let status = Command::new("npm")
            .args(["install", "--save-dev", "husky"])
            .current_dir(&self.project_dir)
            .status()
            .map_err(HookError::Io)?;
        if !status.success() {
            return Err(HookError::InstallFailed(format!(
                "npm install --save-dev husky exited with {}",
                status.code().unwrap_or(-1)
            )));
        }

        // `npx husky install` writes `.husky/_/` and sets
        // `git config core.hooksPath .husky/_`. Idempotent.
        let status = Command::new("npx")
            .args(["--yes", "husky", "install"])
            .current_dir(&self.project_dir)
            .status()
            .map_err(HookError::Io)?;
        if !status.success() {
            return Err(HookError::InstallFailed(format!(
                "npx husky install exited with {}",
                status.code().unwrap_or(-1)
            )));
        }

        if crate::observability::telemetry_gate::is_enabled() {
            tracing::info!(event = "git_hooks.installed", framework = "husky");
        }
        Ok(())
    }

    pub fn update(&self) -> Result<(), HookError> {
        if !npx_available() {
            return Err(HookError::FrameworkNotInstalled("npx".to_string()));
        }
        let status = Command::new("npm")
            .args(["install", "--save-dev", "husky@latest"])
            .current_dir(&self.project_dir)
            .status()
            .map_err(HookError::Io)?;
        if !status.success() {
            return Err(HookError::UpdateFailed(format!(
                "npm install husky@latest exited with {}",
                status.code().unwrap_or(-1)
            )));
        }
        // Re-run install to refresh .husky/_/ scaffolding.
        self.install()?;
        if crate::observability::telemetry_gate::is_enabled() {
            tracing::info!(event = "git_hooks.updated", framework = "husky");
        }
        Ok(())
    }

    /// Husky doesn't have a built-in "run every hook" CLI like
    /// `pre-commit run --all-files`. We iterate `.husky/<name>`
    /// executable files (skipping the `_/` scaffolding subdir) and
    /// invoke each in turn. If `hook_id` is supplied, run only that
    /// one. `all_files` is accepted for API parity but ignored —
    /// husky hooks decide what to scan themselves.
    pub fn run(&self, _all_files: bool, hook_id: Option<&str>) -> Result<(), HookError> {
        let husky_dir = self.project_dir.join(".husky");
        if !husky_dir.is_dir() {
            return Err(HookError::Config(format!(
                "no .husky/ directory at {}; run `jarvy hooks install` first",
                husky_dir.display()
            )));
        }

        let hooks = collect_husky_hooks(&husky_dir)?;
        let to_run: Vec<&HuskyHookFile> = match hook_id {
            Some(id) => hooks.iter().filter(|h| h.name == id).collect(),
            None => hooks.iter().collect(),
        };

        if to_run.is_empty() {
            if hook_id.is_some() {
                return Err(HookError::RunFailed(format!(
                    "no husky hook named `{}` in {}",
                    hook_id.unwrap_or(""),
                    husky_dir.display()
                )));
            }
            return Ok(());
        }

        let mut had_failure = false;
        for hook in to_run {
            let status = Command::new("sh")
                .arg(&hook.path)
                .current_dir(&self.project_dir)
                .status()
                .map_err(HookError::Io)?;
            if !status.success() {
                had_failure = true;
                eprintln!(
                    "  husky hook `{}` exited with {}",
                    hook.name,
                    status.code().unwrap_or(-1)
                );
            }
        }
        if had_failure {
            return Err(HookError::RunFailed(
                "one or more husky hooks failed".to_string(),
            ));
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<HookInfo>, HookError> {
        let husky_dir = self.project_dir.join(".husky");
        if !husky_dir.is_dir() {
            return Ok(Vec::new());
        }
        let hooks = collect_husky_hooks(&husky_dir)?;
        Ok(hooks
            .into_iter()
            .map(|h| HookInfo {
                id: h.name.clone(),
                repo: "local".to_string(),
                version: String::new(),
                hook_type: h.name,
            })
            .collect())
    }
}

struct HuskyHookFile {
    name: String,
    path: PathBuf,
}

/// Walk `.husky/` for hook scripts. Excludes the `_/` scaffolding
/// subdir + any dotfile.
fn collect_husky_hooks(husky_dir: &Path) -> Result<Vec<HuskyHookFile>, HookError> {
    let mut hooks = Vec::new();
    for entry in std::fs::read_dir(husky_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        // Husky <= 8 ships its shell helper at `.husky/.husky.sh`
        // (already excluded by dotfile filter). Husky >= 9 writes
        // hooks directly with no extension; we accept anything that
        // isn't a dot file or the `_/` dir.
        hooks.push(HuskyHookFile {
            name: name.to_string(),
            path,
        });
    }
    hooks.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(hooks)
}

fn npx_available() -> bool {
    Command::new("npx")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn list_returns_empty_when_no_husky_dir() {
        let tmp = tempdir().unwrap();
        let handler = HuskyHandler::new(tmp.path().to_path_buf());
        assert!(handler.list().unwrap().is_empty());
    }

    #[test]
    fn list_enumerates_husky_hook_files() {
        let tmp = tempdir().unwrap();
        let husky = tmp.path().join(".husky");
        fs::create_dir(&husky).unwrap();
        fs::write(husky.join("pre-commit"), "#!/bin/sh\necho hi\n").unwrap();
        fs::write(husky.join("commit-msg"), "#!/bin/sh\necho msg\n").unwrap();
        // Scaffolding files / dotfiles must be excluded.
        fs::create_dir(husky.join("_")).unwrap();
        fs::write(husky.join("_/husky.sh"), "").unwrap();
        fs::write(husky.join(".gitignore"), "_/\n").unwrap();

        let handler = HuskyHandler::new(tmp.path().to_path_buf());
        let hooks = handler.list().unwrap();
        let names: Vec<_> = hooks.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(names, vec!["commit-msg", "pre-commit"]);
    }

    #[test]
    fn run_errors_when_husky_dir_missing() {
        let tmp = tempdir().unwrap();
        let handler = HuskyHandler::new(tmp.path().to_path_buf());
        let err = handler.run(false, None).expect_err("must error");
        assert!(matches!(err, HookError::Config(_)), "got {err:?}");
    }

    #[test]
    fn run_unknown_hook_id_errors() {
        let tmp = tempdir().unwrap();
        let husky = tmp.path().join(".husky");
        fs::create_dir(&husky).unwrap();
        fs::write(husky.join("pre-commit"), "#!/bin/sh\nexit 0\n").unwrap();
        let handler = HuskyHandler::new(tmp.path().to_path_buf());
        let err = handler.run(false, Some("ghost")).expect_err("must error");
        assert!(matches!(err, HookError::RunFailed(_)), "got {err:?}");
    }

    /// Husky requires `package.json`. Surfaces as Config error early.
    #[test]
    fn install_requires_package_json() {
        // Only meaningful when npx is available; otherwise the outer
        // guard fires first.
        if !npx_available() {
            return;
        }
        let tmp = tempdir().unwrap();
        let handler = HuskyHandler::new(tmp.path().to_path_buf());
        let err = handler
            .install()
            .expect_err("must error without package.json");
        assert!(matches!(err, HookError::Config(_)), "got {err:?}");
    }
}
