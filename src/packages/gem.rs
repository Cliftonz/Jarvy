//! Ruby gem installation handler
//!
//! Installs Ruby gems via `gem install <name> [-v <version>]`. Targets the
//! user-active `gem` interpreter — version-manager users (rbenv, asdf) get
//! installs into their currently-selected ruby; system-ruby users get a
//! global install (sudo may be required out of band).
//!
//! Lock-file workflows (`bundle install` against a project `Gemfile.lock`)
//! are intentionally out of scope; that's a per-project concern handled by
//! the project's own bootstrap, not by `jarvy setup`.

use super::common::{PackageError, run_install_loop};
use super::config::GemConfig;

/// Handler for Ruby gem installation
pub struct GemHandler {
    config: GemConfig,
}

impl GemHandler {
    /// Create a new gem handler
    pub fn new(config: GemConfig) -> Self {
        Self { config }
    }

    /// Install all configured gems
    pub fn install(&self) -> Result<(), PackageError> {
        run_install_loop(
            "gem",
            "gem",
            "[gem]",
            "No gem packages configured",
            &self.config.packages,
            |name, spec| Ok(build_install_args(name, spec.version())),
        )
    }
}

/// Build the argv passed to `gem`. `--no-document` is set unconditionally
/// — provisioning runs don't need RDoc/RI for global tooling, and skipping
/// the build cuts install time from ~30s to ~3s for chatty gems like
/// `rubocop`. `-v <version>` only when not "latest".
pub(crate) fn build_install_args(name: &str, version: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::with_capacity(5);
    args.push("install".into());
    args.push("--no-document".into());
    args.push(name.into());
    if version != "latest" {
        args.push("-v".into());
        args.push(version.into());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::super::config::PackageSpec;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn gem_handler_empty() {
        let config = GemConfig::default();
        let handler = GemHandler::new(config);
        assert!(handler.config.packages.is_empty());
    }

    #[test]
    fn gem_handler_holds_packages() {
        let mut packages = HashMap::new();
        packages.insert(
            "rubocop".to_string(),
            PackageSpec::Version("latest".to_string()),
        );
        packages.insert(
            "bundler".to_string(),
            PackageSpec::Version("2.5.0".to_string()),
        );
        let config = GemConfig { packages };
        let handler = GemHandler::new(config);
        assert_eq!(handler.config.packages.len(), 2);
    }

    /// Pin the argv contract — `--no-document` must stay (the speed
    /// difference matters), `install` must stay (not `update`, which
    /// errors on first install), and the `-v` form (not `--version`)
    /// matches what `gem` documents.
    #[test]
    fn build_install_args_table() {
        let cases = [
            (
                "rubocop",
                "latest",
                vec!["install", "--no-document", "rubocop"],
            ),
            (
                "bundler",
                "2.5.0",
                vec!["install", "--no-document", "bundler", "-v", "2.5.0"],
            ),
        ];
        for (name, version, expected) in cases {
            let actual = build_install_args(name, version);
            let actual_refs: Vec<&str> = actual.iter().map(String::as_str).collect();
            assert_eq!(
                actual_refs, expected,
                "argv mismatch for {} = {}",
                name, version
            );
            assert_eq!(actual[0], "install");
            assert_eq!(actual[1], "--no-document");
        }
    }

    /// Flag-like gem names must be refused before they hit `gem`. The
    /// shared `validate_package_name` (called by `run_install_loop`) is
    /// covered exhaustively in `common::tests`; this asserts the gem
    /// handler wires through to it.
    #[test]
    fn gem_rejects_flag_like_names() {
        use super::super::common::validate_package_name;
        let err =
            validate_package_name("--source", "[gem]").expect_err("flag-like name must be refused");
        assert!(matches!(err, PackageError::RefusedUnsafeSpec(_, _)));
    }
}
