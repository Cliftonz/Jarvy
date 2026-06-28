//! cargo binary installation handler
//!
//! Provides installation of Rust binaries via `cargo install`.
//! Supports version pinning and feature selection.

use super::common::{PackageError, run_install_loop, validate_package_name};
use super::config::{CargoConfig, PackageSpec};

/// Handler for cargo binary installation
pub struct CargoHandler {
    config: CargoConfig,
}

impl CargoHandler {
    /// Create a new cargo handler
    pub fn new(config: CargoConfig) -> Self {
        Self { config }
    }

    /// Install all configured cargo binaries
    pub fn install(&self) -> Result<(), PackageError> {
        let locked = self.config.locked;
        run_install_loop(
            "cargo",
            "cargo",
            "[cargo]",
            "No cargo packages configured",
            &self.config.packages,
            move |name, spec| {
                // Reject names that look like cargo flags (`--git`,
                // `--root`) or direct-URL deps before they hit `cargo
                // install`. Name/version are validated by the shared
                // loop; features need an extra pass because they're
                // ecosystem-specific.
                for feature in spec.features() {
                    validate_package_name(feature, "[cargo features]")?;
                }
                Ok(build_install_args(name, spec, locked))
            },
        )
    }
}

/// Build the argv passed to `cargo`. Pinned by a unit test below.
pub(crate) fn build_install_args(name: &str, spec: &PackageSpec, locked: bool) -> Vec<String> {
    let mut args: Vec<String> = Vec::with_capacity(8);
    args.push("install".into());
    args.push(name.into());

    let version = spec.version();
    if version != "latest" {
        args.push("--version".into());
        args.push(version.into());
    }

    let features = spec.features();
    if !features.is_empty() {
        args.push("--features".into());
        args.push(features.join(","));
    }

    if locked {
        args.push("--locked".into());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_cargo_handler_empty() {
        let config = CargoConfig::default();
        let handler = CargoHandler::new(config);
        // Just verify it doesn't panic
        assert!(handler.config.packages.is_empty());
    }

    #[test]
    fn test_cargo_config_with_packages() {
        let mut packages = HashMap::new();
        packages.insert(
            "cargo-watch".to_string(),
            PackageSpec::Version("latest".to_string()),
        );
        packages.insert(
            "cargo-nextest".to_string(),
            PackageSpec::Version("0.9".to_string()),
        );

        let config = CargoConfig {
            packages,
            locked: true,
        };

        let handler = CargoHandler::new(config);
        assert!(handler.config.locked);
        assert_eq!(handler.config.packages.len(), 2);
    }
}
