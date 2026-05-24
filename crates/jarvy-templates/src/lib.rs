//! Shared scaffold template + tool-name validation.
//!
//! Single source of truth for the `define_tool!` macro stub and the
//! `[A-Za-z0-9._-]{1,64}` validation gate, used by both:
//! - the `jarvy` lib crate (via `jarvy tools --request <name>`)
//! - the `cargo-jarvy` workspace tool (`cargo run -p cargo-jarvy -- new-tool <name>`)
//!
//! Kept dependency-free on purpose: `cargo-jarvy` is meant to be a
//! lightweight scaffolder. Depending on the full `jarvy` crate
//! (tokio, OTEL, serde stack) just to call two pure functions added
//! multi-second build penalty for no benefit. This crate has zero
//! runtime deps so dependent build times stay sub-second.

#![forbid(unsafe_code)]

/// Maximum tool-name length accepted by [`validate_tool_name`]. 64 is
/// enough for every real tool in the registry and short enough to
/// keep telemetry attributes / log lines from blowing up.
pub const MAX_TOOL_NAME_LEN: usize = 64;

/// Strict validation for tool names that flow into source-code
/// scaffolding or external commands. Returns `Ok` only for the
/// conservative shape `[A-Za-z0-9._-]{1,64}` AND with at least one
/// alphanumeric byte AND not equal to a filesystem-traversal token.
///
/// Used to gate paths where an unvalidated name would be a real
/// injection risk:
/// - [`render_tool_template`] output is paste-into-Rust-source — a
///   name like `foo"); std::process::exit(0); ("` would escape the
///   embedded string literal.
/// - `cargo-jarvy new-tool` calls `fs::create_dir_all` on the name —
///   `.` / `..` / `...` would land in `src/tools/` / `src/` / etc.
/// - All-punctuation names (`...`, `___`) would produce broken Rust
///   module names.
pub fn validate_tool_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("tool name must not be empty");
    }
    if name.len() > MAX_TOOL_NAME_LEN {
        return Err("tool name too long (max 64 bytes)");
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err("tool name must match [A-Za-z0-9._-]");
    }
    if matches!(name, "." | "..") {
        return Err("tool name must not be a filesystem traversal token");
    }
    if !name.bytes().any(|b| b.is_ascii_alphanumeric()) {
        return Err("tool name must contain at least one alphanumeric byte");
    }
    Ok(())
}

/// Render the canonical `define_tool!` template with placeholder
/// substitution for a new tool.
///
/// Callers MUST validate the name with [`validate_tool_name`] first;
/// this function does not re-check (idempotent / pure substitution).
///
/// `bin` defaults to `name`. The template file lives at
/// `src/template.rs` and is the byte-identical content
/// `cargo-jarvy new-tool` writes to disk plus `jarvy tools --request`
/// prints as a snippet — drift between those two surfaces was the
/// reason this crate exists.
pub fn render_tool_template(name: &str, bin: Option<&str>) -> String {
    const TEMPLATE: &str = include_str!("template.rs");
    let bin = bin.unwrap_or(name);
    // upper-with-underscore for the static identifier — Rust idents
    // can't contain `-` so the hyphen has to be folded.
    let upper = name.to_ascii_uppercase().replace('-', "_");
    let desc = format!("{} tool", name);
    let winget_id = format!("Publisher.{}", name);
    TEMPLATE
        .replace("__TOOL_MOD__", name)
        .replace("__TOOL_BIN__", bin)
        .replace("__TOOL_UPPER__", &upper)
        .replace("__TOOL_DESC__", &desc)
        .replace("__PKG_BREW__", name)
        .replace("__PKG_LINUX__", name)
        .replace("__PKG_WINGET_ID__", &winget_id)
        .replace("__PKG_BSD__", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- validate_tool_name ----------

    #[test]
    fn validate_accepts_canonical() {
        assert!(validate_tool_name("git").is_ok());
        assert!(validate_tool_name("docker-compose").is_ok());
        assert!(validate_tool_name("k3s.io").is_ok());
        assert!(validate_tool_name("my_tool_2").is_ok());
    }

    #[test]
    fn validate_rejects_empty() {
        assert!(validate_tool_name("").is_err());
    }

    #[test]
    fn validate_rejects_too_long() {
        assert!(validate_tool_name(&"a".repeat(MAX_TOOL_NAME_LEN + 1)).is_err());
        assert!(validate_tool_name(&"a".repeat(MAX_TOOL_NAME_LEN)).is_ok());
    }

    #[test]
    fn validate_rejects_injection_chars() {
        assert!(validate_tool_name("foo\"); panic!(\"x").is_err());
        assert!(validate_tool_name("foo bar").is_err());
        assert!(validate_tool_name("foo\nbar").is_err());
        assert!(validate_tool_name("foo;rm").is_err());
        assert!(validate_tool_name("foo|sh").is_err());
        assert!(validate_tool_name("foo&calc").is_err());
    }

    #[test]
    fn validate_rejects_filesystem_traversal() {
        assert!(validate_tool_name(".").is_err());
        assert!(validate_tool_name("..").is_err());
    }

    #[test]
    fn validate_rejects_all_punctuation_names() {
        assert!(validate_tool_name("...").is_err());
        assert!(validate_tool_name("---").is_err());
        assert!(validate_tool_name("___").is_err());
        assert!(validate_tool_name("_._").is_err());
    }

    #[test]
    fn validate_rejects_unicode_homoglyphs() {
        // Cyrillic 'г' looks like Latin 'g' — byte check rejects
        // because it's non-ASCII.
        assert!(validate_tool_name("гit").is_err());
    }

    // ---------- render_tool_template ----------

    #[test]
    fn render_no_placeholders_survive() {
        let out = render_tool_template("mytool", None);
        assert!(!out.contains("__TOOL_"));
        assert!(!out.contains("__PKG_"));
    }

    #[test]
    fn render_substitutes_bsd_pkg() {
        // The drift fix that motivated this crate. cargo-jarvy
        // previously didn't substitute __PKG_BSD__.
        let out = render_tool_template("mytool", None);
        assert!(
            out.contains(r#"bsd: { pkg: "mytool" }"#),
            "BSD substitution: {}",
            out
        );
    }

    #[test]
    fn render_some_bin_differs_from_none() {
        let with_bin = render_tool_template("mytool", Some("mt"));
        let without = render_tool_template("mytool", None);
        assert!(with_bin.contains(r#"command: "mt""#));
        assert!(without.contains(r#"command: "mytool""#));
        assert_ne!(with_bin, without);
    }

    #[test]
    fn render_upper_replaces_hyphens() {
        let out = render_tool_template("docker-compose", None);
        assert!(out.contains("define_tool!(DOCKER_COMPOSE,"), "got: {}", out);
    }

    #[test]
    fn render_winget_id_is_publisher_dot_name() {
        let out = render_tool_template("mytool", None);
        assert!(out.contains(r#"winget: "Publisher.mytool""#));
    }
}
