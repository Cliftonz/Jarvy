//! Go binary installation handler
//!
//! Installs Go binaries via `go install <module>@<version>`. Targets the
//! user's `GOBIN` (or `$GOPATH/bin`, or `$HOME/go/bin` fallback) — the
//! same path `go install` itself uses. Lock-file workflows (`go.mod`
//! resolution in a project tree) are intentionally out of scope.

use super::common::{PackageError, run_install_loop};
use super::config::GoConfig;

/// Handler for Go binary installation
pub struct GoHandler {
    config: GoConfig,
}

impl GoHandler {
    /// Create a new go handler
    pub fn new(config: GoConfig) -> Self {
        Self { config }
    }

    /// Install all configured go binaries
    pub fn install(&self) -> Result<(), PackageError> {
        run_install_loop(
            "go",
            "go",
            "[go]",
            "No go packages configured",
            &self.config.packages,
            |name, spec| {
                Ok(vec![
                    "install".to_string(),
                    build_module_spec(name, spec.version()),
                ])
            },
        )
    }
}

/// Build the `<module>@<version>` argument that `go install` requires.
/// Go's tooling treats `@latest` and `@<semver>` as documented inputs —
/// no version implies module-graph resolution that only works inside a
/// `go.mod` tree, which is not the global-install path users want here.
pub(crate) fn build_module_spec(name: &str, version: &str) -> String {
    if version == "latest" {
        format!("{name}@latest")
    } else {
        format!("{name}@{version}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::PackageSpec;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn go_handler_empty() {
        let config = GoConfig::default();
        let handler = GoHandler::new(config);
        assert!(handler.config.packages.is_empty());
    }

    #[test]
    fn go_handler_holds_packages() {
        let mut packages = HashMap::new();
        packages.insert(
            "github.com/golangci/golangci-lint/cmd/golangci-lint".to_string(),
            PackageSpec::Version("latest".to_string()),
        );
        packages.insert(
            "github.com/cosmtrek/air".to_string(),
            PackageSpec::Version("v1.49.0".to_string()),
        );
        let config = GoConfig { packages };
        let handler = GoHandler::new(config);
        assert_eq!(handler.config.packages.len(), 2);
    }

    /// `<module>@<version>` is mandatory for `go install` outside a
    /// `go.mod` tree — pin the contract.
    #[test]
    fn build_module_spec_table() {
        assert_eq!(
            build_module_spec("github.com/cosmtrek/air", "latest"),
            "github.com/cosmtrek/air@latest"
        );
        assert_eq!(
            build_module_spec("golang.org/x/tools/gopls", "v0.15.0"),
            "golang.org/x/tools/gopls@v0.15.0"
        );
    }

    /// Flag-like go module paths must be refused before they hit `go`.
    /// Wiring asserted via the shared validator; full coverage in
    /// `common::tests`.
    #[test]
    fn go_rejects_flag_like_names() {
        use super::super::common::validate_package_name;
        let err = validate_package_name("--mod", "[go]")
            .expect_err("flag-like module path must be refused");
        assert!(matches!(err, PackageError::RefusedUnsafeSpec(_, _)));
    }
}
