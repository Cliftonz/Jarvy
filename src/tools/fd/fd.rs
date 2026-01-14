//! fd - fast and user-friendly alternative to find
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(FD, {
    command: "fd",
    macos: { brew: "fd" },
    linux: { apt: "fd-find", dnf: "fd-find", pacman: "fd", apk: "fd" },
    windows: { winget: "sharkdp.fd" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_fd_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
