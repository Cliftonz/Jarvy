//! Remote-registry manifest schema.
//!
//! The manifest lists every tool TOML the registry publishes, along with
//! a sha256 of each so Jarvy can refuse a swap-out attack on individual
//! TOML downloads. The manifest itself is cosign-signed (companion
//! `.sig` + `.pem` files at the same URL).
//!
//! Format (JSON):
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "generated_at": "2026-06-22T20:00:00Z",
//!   "tools": [
//!     {
//!       "name": "tailscale-extra",
//!       "path": "tools/tailscale-extra.toml",
//!       "sha256": "abc123..."
//!     }
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error;

/// Current manifest schema version. Jarvy refuses to load a manifest
/// claiming a higher version than this — bumping the constant requires a
/// CLI release that knows how to parse the new shape.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest parse failed: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("manifest body is not valid utf-8")]
    InvalidEncoding,
    #[error(
        "manifest schema version {found} is unsupported (expected {supported}); \
         {hint}"
    )]
    UnsupportedSchema {
        found: u32,
        supported: u32,
        hint: &'static str,
    },
    #[error("manifest tool entry {name:?} has invalid path {path:?}: {reason}")]
    InvalidPath {
        name: String,
        path: String,
        reason: &'static str,
    },
    #[error("manifest tool entry {name:?} has invalid sha256 (must be lowercase 64-char hex)")]
    InvalidSha256 { name: String },
    #[error("manifest tool entry {name:?} has invalid name (must match [a-z0-9_-]+)")]
    InvalidName { name: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    #[serde(default)]
    pub generated_at: Option<String>,
    pub tools: Vec<ToolEntry>,
}

/// Public manifest tool entry. Field validation runs during deserialize
/// (single pass) via the `#[serde(try_from)]` adapter below; typed
/// errors are recovered through the `LAST_VALIDATION_ERROR` thread-local
/// sidechannel. Direct construction in test code goes through
/// [`ToolEntry::for_test`] which skips validation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "RawToolEntry")]
pub struct ToolEntry {
    pub name: String,
    pub path: String,
    pub sha256: String,
}

/// Verbatim shape of a tool entry as it appears in the manifest JSON.
/// Used only as a deserialize-target; the `TryFrom` impl below converts
/// it to a validated `ToolEntry`.
#[derive(Deserialize)]
struct RawToolEntry {
    name: String,
    path: String,
    sha256: String,
}

thread_local! {
    /// Per-thread sidechannel for typed `ManifestError` values raised
    /// during deserialization. Set by `TryFrom<RawToolEntry>`; drained
    /// by `Manifest::parse` after `serde_json::from_str` returns.
    ///
    /// **Why this exists.** Serde's `try_from` attribute requires the
    /// `TryFrom::Error` type to implement `Display`; serde then converts
    /// it via `<D::Error as serde::de::Error>::custom(error)`, which
    /// flattens the error into a string. There is no public API to
    /// recover the original typed error from a `serde_json::Error` —
    /// the variant is structurally erased. The sidechannel preserves
    /// the typed variant so `Manifest::parse` can return the same
    /// `ManifestError::InvalidName/Path/Sha256` shape it did before the
    /// switch to single-pass validation. Per-thread by design — `parse`
    /// is sync and runs on the calling thread, so each thread has its
    /// own cell and there's no cross-thread leak.
    static LAST_VALIDATION_ERROR: RefCell<Option<ManifestError>> =
        const { RefCell::new(None) };
}

fn stash_validation_error(e: ManifestError) -> String {
    let msg = e.to_string();
    LAST_VALIDATION_ERROR.with(|cell| {
        // First-error-wins: if a prior entry already failed in this
        // parse call, keep its error. Matches the pre-refactor
        // "first invalid entry aborts" behavior.
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(e);
        }
    });
    msg
}

impl TryFrom<RawToolEntry> for ToolEntry {
    /// `String` (which is `Display`) so serde can wrap us in
    /// `D::Error::custom`. The typed error is in the thread-local cell.
    type Error = String;

    fn try_from(raw: RawToolEntry) -> Result<Self, Self::Error> {
        if let Err(e) = validate_name(&raw.name) {
            return Err(stash_validation_error(e));
        }
        if let Err(e) = validate_path(&raw.name, &raw.path) {
            return Err(stash_validation_error(e));
        }
        if let Err(e) = validate_sha256(&raw.name, &raw.sha256) {
            return Err(stash_validation_error(e));
        }
        Ok(ToolEntry {
            name: raw.name,
            path: raw.path,
            sha256: raw.sha256,
        })
    }
}

#[cfg(test)]
impl ToolEntry {
    /// Test-only direct constructor that skips validation. Used by
    /// fixtures that already know the shape is valid.
    #[allow(dead_code)]
    pub(crate) fn for_test(name: &str, path: &str, sha256: &str) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            sha256: sha256.into(),
        }
    }
}

impl Manifest {
    /// Parse a manifest body. Validates schema version + every entry's
    /// fields in a single pass — entry validation runs inside
    /// `TryFrom<RawToolEntry>` during deserialization, so a malformed
    /// manifest bails on the first bad row without ever populating the
    /// `tools` vec. A typed `ManifestError::InvalidName/Path/Sha256`
    /// surfaces via the thread-local sidechannel; only a true JSON
    /// shape error returns `ManifestError::Parse`.
    pub fn parse(body: &str) -> Result<Self, ManifestError> {
        // Drain the sidechannel BEFORE the parse so any leftover from a
        // panicked prior call doesn't bleed into this one.
        LAST_VALIDATION_ERROR.with(|cell| {
            cell.borrow_mut().take();
        });

        let result = serde_json::from_str::<Manifest>(body);

        // If the parse failed, prefer a stashed typed error over the
        // opaque `serde_json::Error`. The `take()` clears the cell so
        // subsequent calls start clean.
        match result {
            Ok(manifest) => {
                // Defensive: a successful parse with no typed error
                // should never have left anything in the cell, but
                // clear it anyway.
                LAST_VALIDATION_ERROR.with(|cell| {
                    cell.borrow_mut().take();
                });

                // schema_version == 0 is reserved (sentinel for any
                // future "draft / do-not-load"); refuse explicitly so
                // the current SUPPORTED_SCHEMA_VERSION isn't accidentally
                // compatible with a zero-valued draft manifest.
                if manifest.schema_version == 0 {
                    return Err(ManifestError::UnsupportedSchema {
                        found: 0,
                        supported: SUPPORTED_SCHEMA_VERSION,
                        hint: "schema_version 0 is reserved; the registry must set a positive version",
                    });
                }
                if manifest.schema_version > SUPPORTED_SCHEMA_VERSION {
                    return Err(ManifestError::UnsupportedSchema {
                        found: manifest.schema_version,
                        supported: SUPPORTED_SCHEMA_VERSION,
                        hint: "upgrade jarvy to use this registry",
                    });
                }
                Ok(manifest)
            }
            Err(serde_err) => {
                if let Some(typed) = LAST_VALIDATION_ERROR.with(|cell| cell.borrow_mut().take()) {
                    Err(typed)
                } else {
                    Err(ManifestError::Parse(serde_err))
                }
            }
        }
    }
}

/// Allowed tool-name pattern. Mirrors `crate::tools::plugins`'s identifier
/// validation so a remote-synced tool name is always a valid plugin
/// filename stem.
fn validate_name(name: &str) -> Result<(), ManifestError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(ManifestError::InvalidName {
            name: name.to_string(),
        });
    }
    Ok(())
}

/// A manifest path must be a relative reference under the registry root.
/// Refuse:
///
/// - Absolute paths (`/etc/passwd`)
/// - URLs (`https://attacker.example/...`)
/// - Directory traversal (`../../`)
/// - Backslashes (Windows path separators that could confuse extract)
fn validate_path(name: &str, path: &str) -> Result<(), ManifestError> {
    let invalid_reason = if path.is_empty() {
        Some("empty path")
    } else if path.starts_with('/') {
        Some("must be relative, not absolute")
    } else if path.contains("..") {
        Some("must not contain `..` traversal segments")
    } else if path.contains('\\') {
        Some("must not contain backslashes")
    } else if path.contains("://") {
        Some("must be a relative path, not a URL")
    } else if !path.ends_with(".toml") {
        Some("must end in `.toml`")
    } else {
        None
    };

    if let Some(reason) = invalid_reason {
        return Err(ManifestError::InvalidPath {
            name: name.to_string(),
            path: path.to_string(),
            reason,
        });
    }
    Ok(())
}

fn validate_sha256(name: &str, sha: &str) -> Result<(), ManifestError> {
    if sha.len() != 64
        || !sha
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(ManifestError::InvalidSha256 {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_sha() -> &'static str {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }

    #[test]
    fn accepts_minimal_valid_manifest() {
        let body = format!(
            r#"{{
              "schema_version": 1,
              "tools": [
                {{
                  "name": "foo",
                  "path": "tools/foo.toml",
                  "sha256": "{}"
                }}
              ]
            }}"#,
            valid_sha()
        );
        let m = Manifest::parse(&body).expect("should parse");
        assert_eq!(m.tools.len(), 1);
        assert_eq!(m.tools[0].name, "foo");
    }

    #[test]
    fn rejects_newer_schema_version() {
        let body = format!(
            r#"{{"schema_version": 99, "tools": [{{"name": "f", "path": "tools/f.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(
            err,
            ManifestError::UnsupportedSchema { found: 99, .. }
        ));
    }

    #[test]
    fn rejects_schema_version_zero() {
        let body = format!(
            r#"{{"schema_version": 0, "tools": [{{"name": "f", "path": "tools/f.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(
            err,
            ManifestError::UnsupportedSchema { found: 0, .. }
        ));
    }

    #[test]
    fn rejects_path_with_dotdot_traversal() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "../etc/passwd.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidPath { .. }));
    }

    #[test]
    fn rejects_absolute_path() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "/etc/passwd.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidPath { .. }));
    }

    #[test]
    fn rejects_url_in_path() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "https://attacker.example/x.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidPath { .. }));
    }

    #[test]
    fn rejects_uppercase_sha() {
        let upper = valid_sha().to_uppercase();
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "tools/f.toml", "sha256": "{}"}}]}}"#,
            upper
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidSha256 { .. }));
    }

    #[test]
    fn rejects_short_sha() {
        let body = r#"{"schema_version": 1, "tools": [{"name": "f", "path": "tools/f.toml", "sha256": "abc"}]}"#;
        let err = Manifest::parse(body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidSha256 { .. }));
    }

    #[test]
    fn rejects_invalid_tool_name() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "Bad/Name", "path": "tools/f.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidName { .. }));
    }

    #[test]
    fn rejects_non_toml_path() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "tools/f.json", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidPath { .. }));
    }

    /// Pin current behavior for `./tools/foo.toml`. The validator
    /// currently accepts a leading `./`. If a future tightening rejects
    /// it, this test moves the assertion side; either way the call site
    /// is documented.
    #[test]
    fn accepts_leading_dot_slash_path() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "./tools/f.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let m = Manifest::parse(&body).expect("./prefix is currently accepted");
        assert_eq!(m.tools.len(), 1);
    }

    /// Manifest with two entries sharing the same `name`: parse SUCCEEDS
    /// (the validator doesn't dedupe). Last-wins is the sync orchestrator's
    /// problem — the HashMap insert at the loader side resolves. Pin so
    /// that if a future change rejects duplicates, callers know.
    #[test]
    fn accepts_duplicate_tool_names_at_parse_time() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [
              {{"name": "dup", "path": "tools/dup-a.toml", "sha256": "{}"}},
              {{"name": "dup", "path": "tools/dup-b.toml", "sha256": "{}"}}
            ]}}"#,
            valid_sha(),
            valid_sha()
        );
        let m = Manifest::parse(&body).expect("dedupe is not a parse-layer concern");
        assert_eq!(m.tools.len(), 2);
    }

    /// Very long tool name: regex caps at character set, not length.
    /// Pin that there's no length cap so a future change is intentional.
    #[test]
    fn accepts_arbitrarily_long_name() {
        let long_name = "a".repeat(2048);
        let body = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "{long_name}", "path": "tools/x.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let m = Manifest::parse(&body).expect("no length cap on tool name today");
        assert_eq!(m.tools[0].name.len(), 2048);
    }

    /// Sidechannel regression: after a parse that failed via the
    /// typed-error path, the thread-local must be drained so the next
    /// parse starts clean. Without the drain, a stale error from the
    /// previous call would surface in place of a fresh `Parse` error.
    #[test]
    fn sidechannel_drains_between_calls() {
        // First call: validation failure stashes a typed error.
        let bad = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "../boom.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let err = Manifest::parse(&bad).unwrap_err();
        assert!(matches!(err, ManifestError::InvalidPath { .. }));

        // Second call: valid body must parse cleanly. If the cell
        // weren't drained, the prior error would resurface (because
        // serde succeeds and the cell holds a stale value).
        let good = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "g", "path": "tools/g.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let m = Manifest::parse(&good).expect("valid body parses despite prior failure");
        assert_eq!(m.tools.len(), 1);
        assert_eq!(m.tools[0].name, "g");

        // Third call: a JSON shape error (not a validator failure)
        // surfaces as `Parse`, not as any stale typed error.
        let shape_err = Manifest::parse("not json at all").unwrap_err();
        assert!(
            matches!(shape_err, ManifestError::Parse(_)),
            "JSON shape error must come through Parse; got {shape_err:?}"
        );
    }

    /// First-error-wins. A manifest with two invalid entries surfaces
    /// the first one's typed variant. Serde stops on the first failure,
    /// so only the first stash survives.
    #[test]
    fn first_invalid_entry_wins() {
        let body = format!(
            r#"{{"schema_version": 1, "tools": [
              {{"name": "ok", "path": "tools/ok.toml", "sha256": "{}"}},
              {{"name": "bad-path", "path": "/abs.toml", "sha256": "{}"}},
              {{"name": "Bad/Name", "path": "tools/x.toml", "sha256": "{}"}}
            ]}}"#,
            valid_sha(),
            valid_sha(),
            valid_sha()
        );
        let err = Manifest::parse(&body).unwrap_err();
        // Second entry fails first (absolute path); the third entry's
        // invalid name never gets a chance to be stashed.
        assert!(
            matches!(err, ManifestError::InvalidPath { ref name, .. } if name == "bad-path"),
            "expected InvalidPath on bad-path entry; got {err:?}"
        );
    }

    /// Sidechannel is per-thread. A worker thread doing its own parse
    /// must not see the foreground thread's stashed error.
    #[test]
    fn sidechannel_is_per_thread() {
        let foreground_bad = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "f", "path": "/abs.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let foreground_err = Manifest::parse(&foreground_bad).unwrap_err();
        assert!(matches!(foreground_err, ManifestError::InvalidPath { .. }));

        let good = format!(
            r#"{{"schema_version": 1, "tools": [{{"name": "g", "path": "tools/g.toml", "sha256": "{}"}}]}}"#,
            valid_sha()
        );
        let handle = std::thread::spawn(move || Manifest::parse(&good));
        let result = handle.join().expect("worker did not panic");
        let manifest = result.expect("worker parse must succeed");
        assert_eq!(manifest.tools.len(), 1);
    }
}
