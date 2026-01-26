//! Sensitive data sanitization for logs
//!
//! Redacts API keys, tokens, passwords, and other sensitive information.

use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

/// Patterns for sensitive data that should be redacted
static PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // API Keys and tokens (generic patterns)
        (
            Regex::new(r#"(?i)(api[_-]?key|apikey)[=:]\s*['"]?([a-zA-Z0-9_-]{20,})['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        (
            Regex::new(r#"(?i)(secret|token)[=:]\s*['"]?([a-zA-Z0-9_-]{20,})['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // Bearer tokens
        (
            Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_.+-]+").unwrap(),
            "Bearer [REDACTED]",
        ),
        // Authorization headers
        (
            Regex::new(r#"(?i)(authorization)[=:]\s*['"]?[^'"\s]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // Passwords
        (
            Regex::new(r#"(?i)(password|passwd|pwd)[=:]\s*['"]?[^'"\s]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // AWS credentials
        (
            Regex::new(r#"(?i)(aws[_-]?access[_-]?key[_-]?id)[=:]\s*['"]?[A-Z0-9]{20}['"]?"#)
                .unwrap(),
            "$1=[REDACTED]",
        ),
        (
            Regex::new(
                r#"(?i)(aws[_-]?secret[_-]?access[_-]?key)[=:]\s*['"]?[a-zA-Z0-9/+=]{40}['"]?"#,
            )
            .unwrap(),
            "$1=[REDACTED]",
        ),
        // GitHub tokens
        (
            Regex::new(r"(?i)(gh[ops]_[a-zA-Z0-9]{36,})").unwrap(),
            "[GITHUB_TOKEN_REDACTED]",
        ),
        (
            Regex::new(r#"(?i)(github[_-]?token)[=:]\s*['"]?[a-zA-Z0-9_]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // npm tokens
        (
            Regex::new(r#"(?i)(npm[_-]?token)[=:]\s*['"]?[a-zA-Z0-9-]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // SSH private keys
        (
            Regex::new(
                r"-----BEGIN [A-Z ]+ PRIVATE KEY-----[\s\S]*?-----END [A-Z ]+ PRIVATE KEY-----",
            )
            .unwrap(),
            "[SSH_KEY_REDACTED]",
        ),
        // Email addresses (partial masking)
        (
            Regex::new(r"([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})").unwrap(),
            "[EMAIL_REDACTED]@$2",
        ),
        // Database connection strings
        (
            Regex::new(r"(?i)(postgres|mysql|mongodb|redis)://[^@]+@").unwrap(),
            "$1://[CREDENTIALS_REDACTED]@",
        ),
        // Generic secrets in JSON
        (
            Regex::new(r#"(?i)"(secret|password|token|key|credential)":\s*"[^"]+""#).unwrap(),
            r#""$1":"[REDACTED]""#,
        ),
    ]
});

/// Home directory pattern for replacement
static HOME_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    dirs::home_dir().map(|home| {
        let escaped = regex::escape(home.to_string_lossy().as_ref());
        Regex::new(&escaped).ok()
    })?
});

/// Sanitizer for removing sensitive data from log output
#[derive(Debug, Clone)]
pub struct Sanitizer {
    /// Whether to replace home directory with ~
    replace_home: bool,
    /// Additional custom patterns to redact
    custom_patterns: Vec<(Regex, String)>,
}

impl Sanitizer {
    /// Create a new sanitizer with default settings
    pub fn new() -> Self {
        Self {
            replace_home: true,
            custom_patterns: Vec::new(),
        }
    }

    /// Set whether to replace home directory paths with ~
    pub fn with_replace_home(mut self, replace: bool) -> Self {
        self.replace_home = replace;
        self
    }

    /// Add a custom pattern to redact
    pub fn with_custom_pattern(mut self, pattern: &str, replacement: &str) -> Self {
        if let Ok(regex) = Regex::new(pattern) {
            self.custom_patterns.push((regex, replacement.to_string()));
        }
        self
    }

    /// Sanitize a string by redacting sensitive data
    pub fn sanitize<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut result = Cow::Borrowed(input);

        // Apply built-in patterns
        for (pattern, replacement) in PATTERNS.iter() {
            if pattern.is_match(&result) {
                result = Cow::Owned(pattern.replace_all(&result, *replacement).to_string());
            }
        }

        // Apply custom patterns
        for (pattern, replacement) in &self.custom_patterns {
            if pattern.is_match(&result) {
                result = Cow::Owned(
                    pattern
                        .replace_all(&result, replacement.as_str())
                        .to_string(),
                );
            }
        }

        // Replace home directory with ~
        if self.replace_home {
            if let Some(ref home_regex) = *HOME_PATTERN {
                if home_regex.is_match(&result) {
                    result = Cow::Owned(home_regex.replace_all(&result, "~").to_string());
                }
            }
        }

        result
    }

    /// Check if a string contains potentially sensitive data
    pub fn contains_sensitive(&self, input: &str) -> bool {
        for (pattern, _) in PATTERNS.iter() {
            if pattern.is_match(input) {
                return true;
            }
        }
        for (pattern, _) in &self.custom_patterns {
            if pattern.is_match(input) {
                return true;
            }
        }
        false
    }
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        let sanitizer = Sanitizer::new();
        let input = "api_key=sk_live_abcdefghijklmnopqrstuvwxyz";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("sk_live"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let sanitizer = Sanitizer::new();
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_redact_password() {
        let sanitizer = Sanitizer::new();
        let input = "password=mysupersecretpassword123";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("mysupersecretpassword123"));
    }

    #[test]
    fn test_redact_github_token() {
        let sanitizer = Sanitizer::new();
        let input = "Using token ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[GITHUB_TOKEN_REDACTED]"));
        assert!(!output.contains("ghp_"));
    }

    #[test]
    fn test_redact_aws_credentials() {
        let sanitizer = Sanitizer::new();
        let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_redact_email() {
        let sanitizer = Sanitizer::new();
        let input = "Contact: john.doe@example.com";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[EMAIL_REDACTED]@example.com"));
        assert!(!output.contains("john.doe@"));
    }

    #[test]
    fn test_redact_database_url() {
        let sanitizer = Sanitizer::new();
        let input = "DATABASE_URL=postgres://user:password@localhost:5432/db";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[CREDENTIALS_REDACTED]@"));
        assert!(!output.contains("user:password"));
    }

    #[test]
    fn test_contains_sensitive() {
        let sanitizer = Sanitizer::new();
        assert!(sanitizer.contains_sensitive("api_key=abcdefghijklmnopqrstuvwxyz"));
        assert!(sanitizer.contains_sensitive("Bearer token123456789"));
        assert!(!sanitizer.contains_sensitive("Just a normal log message"));
    }

    #[test]
    fn test_custom_pattern() {
        let sanitizer =
            Sanitizer::new().with_custom_pattern(r"internal-id-\d+", "[INTERNAL_ID_REDACTED]");
        let input = "Processing internal-id-12345";
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[INTERNAL_ID_REDACTED]"));
        assert!(!output.contains("internal-id-12345"));
    }

    #[test]
    fn test_no_false_positives() {
        let sanitizer = Sanitizer::new();
        let input = "Normal log message with tool=git and version=2.40";
        let output = sanitizer.sanitize(input);
        // Should not redact normal tool and version mentions
        assert!(output.contains("tool=git"));
        assert!(output.contains("version=2.40"));
    }

    #[test]
    fn test_json_secret() {
        let sanitizer = Sanitizer::new();
        let input = r#"{"secret":"my_secret_value","name":"test"}"#;
        let output = sanitizer.sanitize(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("my_secret_value"));
    }
}
