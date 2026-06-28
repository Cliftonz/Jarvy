//! HTTPS-only bounded fetch for library manifests + companion artifacts.
//!
//! Mirrors `crate::registry_remote::fetch` but keeps an independent
//! module so the env-var loopback bypass scopes per-feature. PRD-054
//! library fetches use the `JARVY_LIBRARY_ALLOW_INSECURE_FETCH` env
//! var (loopback only) for integration tests — production users
//! cannot reach for it because tests assert non-loopback URLs are
//! refused even with the env var set.

use crate::net::bounded_fetch::{BoundedFetchConfig, BoundedFetchErrorKind, bounded_fetch};
use thiserror::Error;

/// Manifest response cap. Larger than the tools-registry cap because a
/// library may carry hundreds of inline `bash:` script bodies. Still
/// bounded.
pub const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;

/// Per-companion-artifact cap (a hook script or SKILL.md body fetched
/// via `*_url`).
pub const MAX_ITEM_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("fetch failed for {url}: {message}")]
    Network { url: String, message: String },
    #[error("fetch returned HTTP {status} for {url}")]
    HttpStatus { url: String, status: u16 },
    #[error("response body too large for {url}: capped at {cap} bytes")]
    TooLarge { url: String, cap: u64 },
    #[error("read error for {url}: {source}")]
    Read {
        url: String,
        #[source]
        source: std::io::Error,
    },
    #[error("non-https url refused: {0}")]
    NonHttps(String),
}

/// Fetch a URL into a bounded byte buffer. Refuses non-HTTPS URLs unless
/// the loopback-test bypass (env `JARVY_LIBRARY_ALLOW_INSECURE_FETCH` plus
/// a loopback host) is active. Delegates to `net::bounded_fetch` so the
/// HTTPS-only refusal, bounded-read, and loopback-parser logic stays in
/// exactly one place (maint P1, review item 18).
pub fn fetch_bounded(url: &str, max_bytes: u64) -> Result<Vec<u8>, FetchError> {
    let cfg = BoundedFetchConfig {
        insecure_loopback_env: "JARVY_LIBRARY_ALLOW_INSECURE_FETCH",
    };
    bounded_fetch(url, max_bytes, cfg).map_err(|kind| {
        let redacted = crate::network::redact_credentials(url).into_owned();
        match kind {
            BoundedFetchErrorKind::NonHttps => FetchError::NonHttps(redacted),
            BoundedFetchErrorKind::Network(message) => FetchError::Network {
                url: redacted,
                message,
            },
            BoundedFetchErrorKind::HttpStatus(status) => FetchError::HttpStatus {
                url: redacted,
                status,
            },
            BoundedFetchErrorKind::TooLarge => FetchError::TooLarge {
                url: redacted,
                cap: max_bytes,
            },
            BoundedFetchErrorKind::Read(source) => FetchError::Read {
                url: redacted,
                source,
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_http_url() {
        let err = fetch_bounded("http://example.com/manifest.json", 1024).unwrap_err();
        assert!(matches!(err, FetchError::NonHttps(_)));
    }

    #[test]
    fn refuses_ftp_url() {
        let err = fetch_bounded("ftp://example.com/manifest.json", 1024).unwrap_err();
        assert!(matches!(err, FetchError::NonHttps(_)));
    }
}
