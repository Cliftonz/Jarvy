//! terraform - infrastructure as code tool
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(TERRAFORM, {
    command: "terraform",
    macos: { brew: "terraform" },
    linux: { uniform: "terraform" },
    windows: { winget: "HashiCorp.Terraform" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_terraform_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
