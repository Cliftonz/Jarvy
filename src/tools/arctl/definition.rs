//! arctl - Agent Registry CLI
//!
//! arctl is the CLI for agentregistry, a centralized hub for managing LLMs,
//! Agents, Skills, and MCP Servers. Discover, deploy, run, and manage AI
//! artifacts from connected registries.
//!
//! This tool uses the ToolSpec pattern with a custom installer (no Homebrew formula).

use crate::define_tool;
use crate::tools::common::{InstallError, has, run};

define_tool!(ARCTL, {
    command: "arctl",
    custom_install: install_arctl,
});

fn install_arctl(_min_hint: &str) -> Result<(), InstallError> {
    if has("arctl") {
        return Ok(());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        run(
            "bash",
            &[
                "-c",
                "curl -fsSL https://raw.githubusercontent.com/agentregistry-dev/agentregistry/main/scripts/get-arctl | bash",
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
    fn ensure_arctl_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
