//! Sensitive Data Sanitizer
//!
//! Redacts sensitive information from logs, diagnostic bundles, and exports.
//!
//! ## Redacted Patterns
//!
//! - API keys and tokens
//! - Bearer tokens
//! - Passwords and secrets
//! - Email addresses
//! - Home directory paths (normalized to ~/)
//!
//! ## Usage
//!
//! ```rust
//! use jarvy::observability::Sanitizer;
//!
//! let sanitizer = Sanitizer::new();
//! let clean = sanitizer.sanitize("api_key=secret123");
//! assert!(clean.contains("[REDACTED]"));
//! ```

use regex::Regex;
use std::sync::LazyLock;

/// Compiled regex patterns for sensitive data detection
static PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // API keys and tokens (various formats)
        (
            Regex::new(r#"(?i)(api[_-]?key|api[_-]?token|access[_-]?token|auth[_-]?token|secret[_-]?key|private[_-]?key)[=:\s]+['"]?[\w\-./+=]{8,}['"]?"#).unwrap(),
            "$1=[REDACTED]"
        ),
        // Bearer tokens
        (
            Regex::new(r"(?i)bearer\s+[\w\-._~+/]+=*").unwrap(),
            "Bearer [REDACTED]"
        ),
        // Authorization headers
        (
            Regex::new(r#"(?i)(authorization)[=:\s]+['"]?[\w\s\-._~+/]+=*['"]?"#).unwrap(),
            "$1=[REDACTED]"
        ),
        // Password fields
        (
            Regex::new(r#"(?i)(password|passwd|pwd)[=:\s]+['"]?[^\s'"]+['"]?"#).unwrap(),
            "$1=[REDACTED]"
        ),
        // Secret fields
        (
            Regex::new(r#"(?i)(secret|credential)[=:\s]+['"]?[^\s'"]+['"]?"#).unwrap(),
            "$1=[REDACTED]"
        ),
        // AWS credentials
        (
            Regex::new(r#"(?i)(aws[_-]?access[_-]?key[_-]?id|aws[_-]?secret[_-]?access[_-]?key)[=:\s]+['"]?[\w/+=]+['"]?"#).unwrap(),
            "$1=[REDACTED]"
        ),
        // GitHub tokens
        (
            Regex::new(r"(gh[pous]_[A-Za-z0-9_]{36,})").unwrap(),
            "[GITHUB_TOKEN_REDACTED]"
        ),
        // npm tokens
        (
            Regex::new(r"(npm_[A-Za-z0-9]{36,})").unwrap(),
            "[NPM_TOKEN_REDACTED]"
        ),
        // Email addresses (partial redaction)
        (
            Regex::new(r"[\w.+-]+@[\w.-]+\.\w{2,}").unwrap(),
            "[EMAIL_REDACTED]"
        ),
        // SSH private key content
        (
            Regex::new(r"-----BEGIN[^-]+PRIVATE KEY-----[\s\S]*?-----END[^-]+PRIVATE KEY-----").unwrap(),
            "[PRIVATE_KEY_REDACTED]"
        ),
        // Generic hex tokens (32+ chars)
        (
            Regex::new(r"[0-9a-fA-F]{32,}").unwrap(),
            "[HEX_TOKEN_REDACTED]"
        ),
    ]
});

/// Sanitizer for removing sensitive data
#[derive(Debug, Clone)]
pub struct Sanitizer {
    /// Home directory path to normalize
    home_dir: Option<String>,
    /// Additional custom patterns
    custom_patterns: Vec<(Regex, String)>,
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Sanitizer {
    /// Create a new sanitizer with default patterns
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().map(|p| p.to_string_lossy().to_string());

        Self {
            home_dir,
            custom_patterns: Vec::new(),
        }
    }

    /// Add a custom redaction pattern
    pub fn add_pattern(&mut self, pattern: &str, replacement: &str) -> Result<(), regex::Error> {
        let regex = Regex::new(pattern)?;
        self.custom_patterns.push((regex, replacement.to_string()));
        Ok(())
    }

    /// Sanitize a string by redacting sensitive data
    pub fn sanitize(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Apply home directory normalization first
        if let Some(ref home) = self.home_dir {
            result = result.replace(home, "~");
        }

        // Apply built-in patterns
        for (pattern, replacement) in PATTERNS.iter() {
            result = pattern.replace_all(&result, *replacement).to_string();
        }

        // Apply custom patterns
        for (pattern, replacement) in &self.custom_patterns {
            result = pattern
                .replace_all(&result, replacement.as_str())
                .to_string();
        }

        result
    }

    /// Sanitize environment variables (key-value pairs)
    pub fn sanitize_env(&self, vars: &[(String, String)]) -> Vec<(String, String)> {
        let sensitive_keys = [
            "api_key",
            "api_token",
            "access_token",
            "auth_token",
            "secret",
            "password",
            "passwd",
            "pwd",
            "credential",
            "aws_access_key_id",
            "aws_secret_access_key",
            "github_token",
            "gh_token",
            "npm_token",
            "private_key",
            "ssh_key",
        ];

        vars.iter()
            .map(|(key, value)| {
                let key_lower = key.to_lowercase();
                let is_sensitive = sensitive_keys.iter().any(|&s| key_lower.contains(s));

                if is_sensitive {
                    (key.clone(), "[REDACTED]".to_string())
                } else {
                    (key.clone(), self.sanitize(value))
                }
            })
            .collect()
    }

    /// Sanitize a JSON value recursively
    pub fn sanitize_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => serde_json::Value::String(self.sanitize(s)),
            serde_json::Value::Object(map) => {
                let sanitized: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| {
                        let key_lower = k.to_lowercase();
                        if key_lower.contains("key")
                            || key_lower.contains("token")
                            || key_lower.contains("secret")
                            || key_lower.contains("password")
                            || key_lower.contains("credential")
                        {
                            (
                                k.clone(),
                                serde_json::Value::String("[REDACTED]".to_string()),
                            )
                        } else {
                            (k.clone(), self.sanitize_json(v))
                        }
                    })
                    .collect();
                serde_json::Value::Object(sanitized)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.sanitize_json(v)).collect())
            }
            other => other.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_api_key() {
        let sanitizer = Sanitizer::new();
        let input = "api_key=sk_live_abc123def456";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("abc123"));
    }

    #[test]
    fn test_sanitize_bearer_token() {
        let sanitizer = Sanitizer::new();
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("eyJhbGc"));
    }

    #[test]
    fn test_sanitize_password() {
        let sanitizer = Sanitizer::new();
        let input = "password=mysecretpassword123";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("mysecret"));
    }

    #[test]
    fn test_sanitize_email() {
        let sanitizer = Sanitizer::new();
        let input = "user email: john.doe@example.com";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[EMAIL_REDACTED]"));
        assert!(!output.contains("john.doe"));
    }

    #[test]
    fn test_sanitize_github_token() {
        let sanitizer = Sanitizer::new();
        let input = "GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("REDACTED"));
    }

    #[test]
    fn test_sanitize_home_dir() {
        let sanitizer = Sanitizer::new();
        if let Some(ref home) = sanitizer.home_dir {
            let input = format!("{}/some/path", home);
            let output = sanitizer.sanitize(&input);
            assert!(output.starts_with("~/"));
        }
    }

    #[test]
    fn test_sanitize_env() {
        let sanitizer = Sanitizer::new();
        let vars = vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("API_KEY".to_string(), "secret123".to_string()),
            ("HOME".to_string(), "/Users/test".to_string()),
        ];
        let result = sanitizer.sanitize_env(&vars);

        assert_eq!(result[0].1, "/usr/bin"); // PATH unchanged
        assert_eq!(result[1].1, "[REDACTED]"); // API_KEY redacted
    }

    #[test]
    fn test_sanitize_json() {
        let sanitizer = Sanitizer::new();
        let json = serde_json::json!({
            "name": "test",
            "api_key": "secret123",
            "nested": {
                "token": "abc123"
            }
        });

        let result = sanitizer.sanitize_json(&json);
        assert_eq!(result["api_key"], "[REDACTED]");
        assert_eq!(result["nested"]["token"], "[REDACTED]");
        assert_eq!(result["name"], "test");
    }

    #[test]
    fn test_custom_pattern() {
        let mut sanitizer = Sanitizer::new();
        sanitizer
            .add_pattern(r"custom_\d+", "[CUSTOM_REDACTED]")
            .unwrap();

        let input = "data: custom_12345";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[CUSTOM_REDACTED]"));
    }
}
