//! HTTPS-only bounded fetch for library manifests + companion artifacts.
//!
//! Mirrors `crate::registry_remote::fetch` but keeps an independent
//! module so the env-var loopback bypass scopes per-feature. PRD-054
//! library fetches use the `JARVY_LIBRARY_ALLOW_INSECURE_FETCH` env
//! var (loopback only) for integration tests — production users
//! cannot reach for it because tests assert non-loopback URLs are
//! refused even with the env var set.

use std::io::Read;
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

/// Fetch a URL into a bounded byte buffer. Refuses non-HTTPS URLs
/// unless the loopback-test bypass is active.
pub fn fetch_bounded(url: &str, max_bytes: u64) -> Result<Vec<u8>, FetchError> {
    if !url.starts_with("https://") && !insecure_loopback_allowed(url) {
        return Err(FetchError::NonHttps(
            crate::network::redact_credentials(url).into_owned(),
        ));
    }

    let agent = crate::net::agent::agent();
    let response = agent
        .get(url)
        .header("User-Agent", crate::net::agent::USER_AGENT)
        .call()
        .map_err(|e| FetchError::Network {
            url: crate::network::redact_credentials(url).into_owned(),
            message: e.to_string(),
        })?;

    if response.status() != 200 {
        return Err(FetchError::HttpStatus {
            url: crate::network::redact_credentials(url).into_owned(),
            status: response.status().as_u16(),
        });
    }

    let mut body = response.into_body();
    let reader = body.as_reader();
    let mut limited = reader.take(max_bytes + 1);
    let mut buf = Vec::with_capacity(8 * 1024);
    limited
        .read_to_end(&mut buf)
        .map_err(|e| FetchError::Read {
            url: crate::network::redact_credentials(url).into_owned(),
            source: e,
        })?;

    if buf.len() as u64 > max_bytes {
        return Err(FetchError::TooLarge {
            url: crate::network::redact_credentials(url).into_owned(),
            cap: max_bytes,
        });
    }

    Ok(buf)
}

fn insecure_loopback_allowed(url: &str) -> bool {
    if std::env::var_os("JARVY_LIBRARY_ALLOW_INSECURE_FETCH").is_none() {
        return false;
    }
    is_plain_loopback_http(url)
}

fn is_plain_loopback_http(url: &str) -> bool {
    let Some(after_scheme) = url.strip_prefix("http://") else {
        return false;
    };
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..authority_end];
    if authority.contains('@') {
        return false;
    }
    let host_end = authority.find(':').unwrap_or(authority.len());
    let host = &authority[..host_end];
    matches!(host, "127.0.0.1" | "localhost")
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

    #[test]
    fn loopback_parser_accepts_clean_loopback() {
        assert!(is_plain_loopback_http("http://127.0.0.1:8080/x"));
        assert!(is_plain_loopback_http("http://localhost:8080/x"));
    }

    #[test]
    fn loopback_parser_refuses_userinfo_bypass() {
        assert!(!is_plain_loopback_http(
            "http://127.0.0.1:80@attacker.example/x"
        ));
        assert!(!is_plain_loopback_http("http://user@127.0.0.1:8080/x"));
    }
}
