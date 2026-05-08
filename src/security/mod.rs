//! Cross-cutting security helpers shared across subsystems.
//!
//! Today this is a single helper for warning about world-readable files
//! holding credentials (proxy passwords, generic secrets). Previously each
//! caller (`env::secrets`, `network::config::PasswordSource::File`)
//! reimplemented the same `metadata().permissions().mode() & 0o077` block
//! with subtly different log fields. Future security utilities (path
//! containment, ownership checks) will live here too.

#![allow(dead_code)] // Public API used by env/secrets and network/config

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Warn if `path` has any group / other permissions set (`& 0o077 != 0`).
/// `kind` describes what the file holds for the structured log
/// (`"secret"`, `"proxy_password"`, ...) so support can filter.
///
/// On non-Unix this is a no-op.
pub fn warn_if_world_readable(path: &Path, kind: &'static str) {
    #[cfg(unix)]
    {
        let Ok(metadata) = std::fs::metadata(path) else {
            return;
        };
        let mode = metadata.permissions().mode();
        if mode & 0o077 == 0 {
            return;
        }
        let safe_path = crate::network::redact_home(&path.display().to_string());
        tracing::warn!(
            event = "security.file_permissive_perms",
            kind = kind,
            path = %safe_path,
            mode = format!("{:o}", mode & 0o777),
            "file has permissive permissions; chmod 600 recommended"
        );
    }
    #[cfg(not(unix))]
    {
        let _ = (path, kind);
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn warn_only_emits_for_world_readable() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "x").unwrap();
        // Tighten to 0600 — must not warn.
        let mut perms = tmp.as_file().metadata().unwrap().permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(tmp.path(), perms).unwrap();
        // Just exercise the path without asserting on log output.
        warn_if_world_readable(tmp.path(), "test");
    }
}
