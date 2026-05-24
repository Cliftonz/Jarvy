//! Credential handling for proxy authentication
//!
//! Supports multiple secure methods for retrieving proxy credentials:
//! - Plain text (with security warning)
//! - Environment variable
//! - File path
//! - Interactive prompt

use super::config::{NetworkConfig, ProxyAuth};
use std::io;

/// Inject authentication credentials into a proxy URL
///
/// Takes a proxy URL like "http://proxy:8080" and credentials,
/// returns "http://user:pass@proxy:8080"
#[allow(dead_code)] // Public API for proxy authentication
pub fn inject_credentials(proxy_url: &str, auth: &ProxyAuth) -> Result<String, String> {
    let password = auth.password.resolve()?;

    // Parse the URL to inject credentials
    if let Some(proto_end) = proxy_url.find("://") {
        let protocol = &proxy_url[..proto_end + 3];
        let rest = &proxy_url[proto_end + 3..];

        // URL-encode username and password
        let encoded_user = urlencoding::encode(&auth.username);
        let encoded_pass = urlencoding::encode(&password);

        Ok(format!(
            "{}{}:{}@{}",
            protocol, encoded_user, encoded_pass, rest
        ))
    } else {
        Err(format!("Invalid proxy URL format: {}", proxy_url))
    }
}

/// Prompt user for password interactively with hidden input.
///
/// Uses inquire's Password prompt which masks characters on the terminal.
#[allow(dead_code)] // Public API for interactive password prompting
pub fn prompt_password(prompt: &str) -> io::Result<String> {
    inquire::Password::new(prompt)
        .without_confirmation()
        .prompt()
        .map_err(|e| io::Error::other(e.to_string()))
}

/// Get the proxy URL with credentials injected if authentication is configured
#[allow(dead_code)] // Public API for proxy authentication
pub fn get_authenticated_proxy(
    proxy_url: Option<&String>,
    auth: Option<&ProxyAuth>,
) -> Result<Option<String>, String> {
    match (proxy_url, auth) {
        (Some(url), Some(auth)) => {
            // Check if URL already has credentials
            if url.contains('@') {
                Ok(Some(url.clone()))
            } else {
                inject_credentials(url, auth).map(Some)
            }
        }
        (Some(url), None) => Ok(Some(url.clone())),
        (None, _) => Ok(None),
    }
}

/// Resolve all proxy URLs with authentication for a NetworkConfig
#[allow(dead_code)] // Public API for proxy authentication
pub struct AuthenticatedProxies {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub socks_proxy: Option<String>,
}

impl AuthenticatedProxies {
    /// Create authenticated proxies from NetworkConfig
    #[allow(dead_code)] // Public API for proxy authentication
    pub fn from_config(config: &NetworkConfig) -> Result<Self, String> {
        Ok(Self {
            http_proxy: get_authenticated_proxy(config.http_proxy.as_ref(), config.auth.as_ref())?,
            https_proxy: get_authenticated_proxy(
                config.https_proxy.as_ref(),
                config.auth.as_ref(),
            )?,
            socks_proxy: get_authenticated_proxy(
                config.socks_proxy.as_ref(),
                config.auth.as_ref(),
            )?,
        })
    }
}

// Shared URL encoder lives in `crate::net::url_encode`. Thin wrapper
// keeps the existing call sites (`urlencoding::encode(...)`) compiling
// unchanged — the re-export form runs afoul of the inner-module
// visibility rules, so this delegates instead.
mod urlencoding {
    #[allow(dead_code)] // Called by inject_credentials below.
    pub fn encode(input: &str) -> String {
        crate::net::url_encode::encode_unreserved(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::config::PasswordSource;

    #[test]
    fn test_inject_credentials() {
        let auth = ProxyAuth {
            username: "user".to_string(),
            password: PasswordSource::Plain("pass".to_string()),
        };

        let result = inject_credentials("http://proxy:8080", &auth).unwrap();
        assert_eq!(result, "http://user:pass@proxy:8080");
    }

    #[test]
    fn test_inject_credentials_with_special_chars() {
        let auth = ProxyAuth {
            username: "user@corp".to_string(),
            password: PasswordSource::Plain("p@ss:word".to_string()),
        };

        let result = inject_credentials("http://proxy:8080", &auth).unwrap();
        // Special chars should be URL-encoded
        assert!(result.contains("%40")); // @ is encoded as %40
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("user"), "user");
        assert_eq!(urlencoding::encode("user@corp"), "user%40corp");
        assert_eq!(urlencoding::encode("p@ss:word"), "p%40ss%3Aword");
    }

    #[test]
    fn test_get_authenticated_proxy_no_auth() {
        let url = Some("http://proxy:8080".to_string());
        let result = get_authenticated_proxy(url.as_ref(), None).unwrap();
        assert_eq!(result, Some("http://proxy:8080".to_string()));
    }

    #[test]
    fn test_get_authenticated_proxy_already_has_creds() {
        let url = Some("http://user:pass@proxy:8080".to_string());
        let auth = ProxyAuth {
            username: "other".to_string(),
            password: PasswordSource::Plain("other".to_string()),
        };
        let result = get_authenticated_proxy(url.as_ref(), Some(&auth)).unwrap();
        // Should keep existing credentials
        assert_eq!(result, Some("http://user:pass@proxy:8080".to_string()));
    }
}
