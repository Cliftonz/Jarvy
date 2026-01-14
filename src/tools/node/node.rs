//! node - Node.js JavaScript runtime
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(NODE, {
    command: "node",
    macos: { brew: "node" },
    linux: { uniform: "nodejs" },
    windows: { winget: "OpenJS.NodeJS.LTS" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_node_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
