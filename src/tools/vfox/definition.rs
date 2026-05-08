//! vfox - cross-platform version manager
//!
//! A cross-platform and extendable version manager with support for
//! Java, Node.js, Golang, Python, Flutter, .NET & more.
//!
//! See: https://github.com/version-fox/vfox

use crate::define_tool;

define_tool!(VFOX, {
    command: "vfox",
    macos: { brew: "vfox" },
    linux: { brew: "vfox" },
    windows: { winget: "vfox" },
    bsd: { pkg: "vfox" },
    default_hook_shell_init: ("vfox", "activate"),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_vfox_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
