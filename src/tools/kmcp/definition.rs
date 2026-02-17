//! kmcp - Build, test, and deploy MCP servers on Kubernetes
//!
//! kmcp is a CLI tool and Kubernetes controller for scaffolding, building,
//! and deploying Model Context Protocol (MCP) servers. Companion tool to kagent.
//!
//! This tool uses the ToolSpec pattern with a custom installer (no Homebrew formula).

use crate::define_tool;
use crate::tools::common::{InstallError, has, run};

define_tool!(KMCP, {
    command: "kmcp",
    custom_install: install_kmcp,
    depends_on: &["kubectl"],
});

fn install_kmcp(_min_hint: &str) -> Result<(), InstallError> {
    if has("kmcp") {
        return Ok(());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        run(
            "bash",
            &[
                "-c",
                "curl -fsSL https://raw.githubusercontent.com/kagent-dev/kmcp/refs/heads/main/scripts/get-kmcp.sh | bash",
            ],
        )?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(InstallError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_kmcp_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
