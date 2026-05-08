//! Canonical resolver for `~/.jarvy/...` paths.
//!
//! Previously 22+ subsystems hand-rolled
//! `dirs::home_dir().map(|h| h.join(".jarvy").join("X"))` with four
//! different fallback policies and the literal `".jarvy"` in 60+ places.
//! Moving `~/.jarvy` (e.g. to `~/.local/share/jarvy` per XDG) used to
//! mean touching every site; with this module it's one constant.
//!
//! This is the natural seam for future XDG migration and for a
//! `JARVY_HOME` env override.

#![allow(dead_code)] // Public API; callers migrate incrementally.

use std::path::PathBuf;

/// Internal constant for the base directory name.
const JARVY_DIR: &str = ".jarvy";

/// Returned when `dirs::home_dir()` cannot be resolved (rare; running as
/// `nobody`, certain container images, etc.).
#[derive(Debug, thiserror::Error)]
#[error("cannot determine home directory")]
pub struct NoHomeDir;

/// `~/.jarvy/`. Honors `JARVY_HOME` if set so the user can override the
/// base location for tests and ad-hoc isolation.
pub fn jarvy_home() -> Result<PathBuf, NoHomeDir> {
    if let Ok(custom) = std::env::var("JARVY_HOME") {
        if !custom.trim().is_empty() {
            return Ok(PathBuf::from(custom));
        }
    }
    dirs::home_dir().map(|h| h.join(JARVY_DIR)).ok_or(NoHomeDir)
}

/// `~/.jarvy/config.toml` — global user config.
pub fn config_toml() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("config.toml"))
}

/// `~/.jarvy/logs/`.
pub fn logs_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("logs"))
}

/// `~/.jarvy/tickets/`.
pub fn tickets_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("tickets"))
}

/// `~/.jarvy/cache/`.
pub fn cache_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("cache"))
}

/// `~/.jarvy/cache/configs/` — used by `remote::fetch_remote_config`.
pub fn remote_config_cache_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(cache_dir()?.join("configs"))
}

/// `~/.jarvy/staging/` — pre-verify download landing zone for `update`.
pub fn staging_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("staging"))
}

/// `~/.jarvy/backup/` — pre-update binary copy for rollback.
pub fn backup_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("backup"))
}

/// `~/.jarvy/tools.d/` — user plugin tool definitions.
pub fn plugins_dir() -> Result<PathBuf, NoHomeDir> {
    Ok(jarvy_home()?.join("tools.d"))
}

/// Project-local drift baseline state file: `<project>/.jarvy/state.json`.
pub fn state_json(project: &std::path::Path) -> PathBuf {
    project.join(JARVY_DIR).join("state.json")
}

/// Create `dir` if it doesn't exist; on Unix tighten its mode to 0o700 so
/// staging downloads / ticket bundles aren't readable by other users on a
/// shared host (security review F-15).
pub fn ensure_dir_0700(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jarvy_home_honors_env_override() {
        let prev = std::env::var("JARVY_HOME").ok();
        // SAFETY: scoped restore via Drop guard pattern is overkill for a
        // single-threaded test; restore explicitly at the end.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("JARVY_HOME", "/tmp/jarvy-test-override");
        }
        let p = jarvy_home().unwrap();
        assert_eq!(p, PathBuf::from("/tmp/jarvy-test-override"));

        // Cleanup.
        #[allow(unsafe_code)]
        unsafe {
            match prev {
                Some(v) => std::env::set_var("JARVY_HOME", v),
                None => std::env::remove_var("JARVY_HOME"),
            }
        }
    }

    #[test]
    fn derived_paths_share_jarvy_home() {
        // Don't rely on actual env; just check the suffixes are right.
        // We don't expect this in CI to use JARVY_HOME, so the default
        // ~/.jarvy/<x> shape is what we verify.
        if std::env::var("JARVY_HOME").is_ok() {
            return;
        }
        let home = jarvy_home().unwrap();
        assert!(logs_dir().unwrap().starts_with(&home));
        assert!(tickets_dir().unwrap().starts_with(&home));
        assert!(cache_dir().unwrap().starts_with(&home));
        assert!(staging_dir().unwrap().starts_with(&home));
        assert!(backup_dir().unwrap().starts_with(&home));
        assert!(config_toml().unwrap().starts_with(&home));
    }
}
