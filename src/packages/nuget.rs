//! .NET global tool installation handler
//!
//! Installs .NET global tools via `dotnet tool install -g <name>`. NuGet is
//! the .NET package ecosystem; "tools" here are CLI binaries published as
//! NuGet packages (e.g. `dotnet-ef`, `csharpier`, `dotnet-outdated-tool`).
//!
//! This handler does NOT manage project-level NuGet PackageReferences —
//! those belong in the project's `.csproj`/`Directory.Packages.props` and are
//! restored by `dotnet restore` during build, not by `jarvy setup`.

use super::common::{PackageError, run_install_loop};
use super::config::NugetConfig;

/// Handler for .NET global tool installation
pub struct NugetHandler {
    config: NugetConfig,
}

impl NugetHandler {
    /// Create a new nuget handler
    pub fn new(config: NugetConfig) -> Self {
        Self { config }
    }

    /// Install all configured global tools. Idempotent via `dotnet tool
    /// update -g` (rather than `install -g` which errors when the tool
    /// is already present).
    pub fn install(&self) -> Result<(), PackageError> {
        run_install_loop(
            "nuget",
            "dotnet",
            "[nuget]",
            "No NuGet global tools configured",
            &self.config.packages,
            |name, spec| Ok(build_install_args(name, spec.version())),
        )
    }
}

/// Build the argv passed to `dotnet`. Pinned by a unit test below so
/// the `tool update -g` (idempotent) shape can't silently regress.
pub(crate) fn build_install_args(name: &str, version: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::with_capacity(6);
    args.push("tool".into());
    args.push("update".into());
    args.push("-g".into());
    args.push(name.into());
    if version != "latest" {
        args.push("--version".into());
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
    fn nuget_handler_empty() {
        let config = NugetConfig::default();
        let handler = NugetHandler::new(config);
        assert!(handler.config.packages.is_empty());
    }

    #[test]
    fn nuget_handler_holds_packages() {
        let mut packages = HashMap::new();
        packages.insert(
            "dotnet-ef".to_string(),
            PackageSpec::Version("latest".to_string()),
        );
        packages.insert(
            "csharpier".to_string(),
            PackageSpec::Version("0.30.0".to_string()),
        );
        let config = NugetConfig { packages };
        let handler = NugetHandler::new(config);
        assert_eq!(handler.config.packages.len(), 2);
    }

    /// Pin the argv contract — flipping `update` → `install` or dropping
    /// `-g` would change semantics catastrophically (loses idempotency,
    /// or installs per-project instead of machine-global). This test
    /// makes those regressions impossible to ship silently.
    #[test]
    fn build_install_args_table() {
        let cases = [
            (
                "dotnet-ef",
                "latest",
                vec!["tool", "update", "-g", "dotnet-ef"],
            ),
            (
                "csharpier",
                "0.30.0",
                vec!["tool", "update", "-g", "csharpier", "--version", "0.30.0"],
            ),
            (
                "dotnet-aspnet-codegenerator",
                "8.0.0",
                vec![
                    "tool",
                    "update",
                    "-g",
                    "dotnet-aspnet-codegenerator",
                    "--version",
                    "8.0.0",
                ],
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
            assert_eq!(actual[0], "tool", "first arg must be `tool`");
            assert_eq!(actual[1], "update", "must use `update` for idempotency");
            assert_eq!(actual[2], "-g", "must be global install");
            assert_ne!(actual[1], "install", "`install` errors when present");
        }
    }

    /// Flag-like nuget tool names must be refused before they hit
    /// `dotnet`. Wiring assertion only; full coverage in `common::tests`.
    #[test]
    fn nuget_rejects_flag_like_tool_names() {
        use super::super::common::validate_package_name;
        let err = validate_package_name("--source", "[nuget]")
            .expect_err("flag-like name must be refused");
        assert!(
            matches!(err, PackageError::RefusedUnsafeSpec(_, _)),
            "expected RefusedUnsafeSpec, got {err:?}"
        );
    }
}
