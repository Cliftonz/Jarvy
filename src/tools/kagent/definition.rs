//! kagent - Kubernetes-native AI agent framework
//!
//! kagent (CNCF Sandbox) provides an engine for building, deploying, and managing
//! AI agents on Kubernetes with built-in MCP server tools for K8s, Istio, Helm,
//! Argo, Prometheus, and more.
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(KAGENT, {
    command: "kagent",
    macos: { brew: "kagent" },
    linux: { brew: "kagent" },
    depends_on: &["kubectl"],
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_kagent_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
