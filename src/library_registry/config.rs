//! `library_sources` config shape, shared by every consumer.

use serde::{Deserialize, Serialize};

/// One entry in a consumer's `library_sources` array.
///
/// ```toml
/// [[ai_hooks.library_sources]]
/// url = "https://cdn.myorg.com/jarvy/manifest.json"
/// require_signature = true
/// identity_regexp = "^https://github\\.com/myorg/jarvy-library/.+$"
/// oidc_issuer = "https://token.actions.githubusercontent.com"
/// refresh_interval_secs = 86400
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LibrarySource {
    /// Manifest URL. HTTPS-only. If the URL ends with `/`,
    /// `manifest.json` is appended.
    pub url: String,

    /// Require cosign signature verification. Default `true`.
    /// `false` emits a `library.signature_disabled` warning every
    /// fetch and is intended only for development. Signature
    /// verification is scaffolded but not enforced in v1 — see PRD-054
    /// follow-up phase.
    #[serde(default = "default_require_signature")]
    pub require_signature: bool,

    /// Cosign signing-identity regexp. Required when
    /// `require_signature = true` and signature verification is
    /// enforced. Today this is captured for future use; the v1 fetch
    /// path does not consult it.
    #[serde(default)]
    pub identity_regexp: Option<String>,

    /// Cosign OIDC issuer URL. Same forward-compat note as above.
    #[serde(default)]
    pub oidc_issuer: Option<String>,

    /// How often to refetch the manifest, in seconds. Default 86400
    /// (24h). The disk cache satisfies reads inside this window.
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,

    /// Optional sha256 hex digest of the manifest body. When set,
    /// every sync recomputes the sha and refuses to apply if it
    /// doesn't match. The strongest tamper-evidence available in v1
    /// until cosign enforcement lands — pinning a sha means a
    /// publisher cannot silently re-publish under the same URL and
    /// expect Jarvy to pick up new content without a visible bump.
    /// Review item 13 (P1).
    #[serde(default)]
    pub manifest_sha256: Option<String>,
}

impl LibrarySource {
    /// Minimal constructor for tests + library consumers that build
    /// `LibrarySource` programmatically (no TOML round-trip).
    #[allow(dead_code)] // Used by tests + lib consumers
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            require_signature: default_require_signature(),
            identity_regexp: None,
            oidc_issuer: None,
            refresh_interval_secs: default_refresh_interval(),
            manifest_sha256: None,
        }
    }
}

fn default_require_signature() -> bool {
    true
}

fn default_refresh_interval() -> u64 {
    86_400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_source() {
        let toml_str = r#"url = "https://cdn.example.com/manifest.json""#;
        let src: LibrarySource = toml::from_str(toml_str).unwrap();
        assert_eq!(src.url, "https://cdn.example.com/manifest.json");
        assert!(src.require_signature);
        assert_eq!(src.refresh_interval_secs, 86_400);
    }

    #[test]
    fn parses_full_source() {
        let toml_str = r#"
url = "https://cdn.example.com/manifest.json"
require_signature = false
identity_regexp = "^https://github\\.com/myorg/.+$"
oidc_issuer = "https://token.actions.githubusercontent.com"
refresh_interval_secs = 3600
"#;
        let src: LibrarySource = toml::from_str(toml_str).unwrap();
        assert!(!src.require_signature);
        assert_eq!(
            src.identity_regexp.as_deref(),
            Some(r"^https://github\.com/myorg/.+$")
        );
        assert_eq!(src.refresh_interval_secs, 3600);
    }

    #[test]
    fn rejects_unknown_fields() {
        let toml_str = r#"
url = "https://cdn.example.com/manifest.json"
nonsense_field = true
"#;
        let err = toml::from_str::<LibrarySource>(toml_str).unwrap_err();
        assert!(format!("{err}").contains("nonsense_field"));
    }
}
