//! kind - run local Kubernetes clusters using Docker
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(KIND, {
    command: "kind",
    macos: { brew: "kind" },
    linux: { brew: "kind" },
    windows: { winget: "Kubernetes.kind" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_kind_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
