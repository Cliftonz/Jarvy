//! httpie - User-friendly HTTP client
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(HTTPIE, {
    command: "http",
    macos: { brew: "httpie" },
    linux: { uniform: "httpie" },
    windows: { winget: "HTTPie.HTTPie" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_httpie_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
