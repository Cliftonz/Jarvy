//! dotnet - .NET SDK and runtime
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(DOTNET, {
    command: "dotnet",
    macos: { cask: "dotnet-sdk" },
    linux: { apt: "dotnet-sdk-8.0", dnf: "dotnet-sdk", pacman: "dotnet-sdk", apk: "dotnet-sdk" },
    windows: { winget: "Microsoft.DotNet.SDK.8" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_dotnet_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
