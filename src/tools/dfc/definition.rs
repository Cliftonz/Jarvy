//! dfc - Dockerfile converter for Chainguard images
//!
//! A CLI utility that converts Dockerfiles to use Chainguard Images and APKs.
//! Facilitates migration to secure, minimal base images by automatically
//! replacing standard base images with their Chainguard equivalents.
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(DFC, {
    command: "dfc",
    macos: { brew: "chainguard-dev/tap/dfc" },
    linux: { brew: "chainguard-dev/tap/dfc" },
    // No native Windows support; requires Go installation
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_dfc_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
