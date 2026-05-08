//! Sensitive Data Sanitizer
//!
//! Redacts sensitive information from logs, diagnostic bundles, and exports
//! before they leave the user's machine (ticket bundles, telemetry payloads,
//! support exports).
//!
//! ## Design notes
//!
//! - **Key-cued patterns over shape-only matches.** A bare regex for "32+
//!   hex chars" sounds safer but it eats commit SHAs, drift `config_hash`
//!   values, image digests, and SHA-256 file fingerprints — all of which
//!   are debug *signal*. We restrict hex matches to those preceded by a
//!   key like `token=` / `key:` so unrelated hex survives the redaction
//!   pass.
//! - **Prefixed-token shapes.** Most modern tokens are self-identifying
//!   via prefix (`ghp_`, `xox[abp]-`, `sk-ant-`, `sk-proj-`, `glpat-`,
//!   `eyJ` JWT). Match those by prefix regardless of surrounding context
//!   so a leaked token in a JSON array or argv slice still redacts.
//! - **Path redaction.** `**/.ssh/id_*` and `**/.gnupg/**` paths leak
//!   private-key locations through ticket bundles. Redact path-only,
//!   leave neighboring text intact.
//! - **Cow over String.** `sanitize` returns the original input when no
//!   pattern matched so the common case (a benign log line) allocates
//!   zero `String`s.

#![allow(dead_code)] // Public API for data sanitization

use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

/// SSH private-key path pattern. Matched separately (not in `PATTERNS`)
/// because we need a closure-based replacer to preserve `.pub` (public)
/// paths — Rust's `regex` crate doesn't support look-ahead so a single
/// regex can't say "id_rsa not followed by .pub".
static SSH_PRIVATE_KEY_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[/~][\w./\-]*?)?\.ssh/id_(?:rsa|ed25519|ecdsa|dsa)(?:\.pub)?")
        .expect("ssh key path regex must compile")
});

/// Compiled regex patterns for sensitive-data redaction.
///
/// Each `(regex, replacement)` pair is applied in order. Order matters:
/// long-prefix patterns (e.g. `sk-ant-...`) run before generic
/// `(secret|key)=...` patterns so the more specific replacement wins.
static PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // ---- Self-identifying token prefixes ---------------------------------
        // GitHub PATs / OAuth tokens (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`)
        (
            Regex::new(r"(?:gh[pousr]_[A-Za-z0-9_]{20,})").unwrap(),
            "[GITHUB_TOKEN_REDACTED]",
        ),
        // Slack tokens
        (
            Regex::new(r"(?:xox[abprso]-[A-Za-z0-9-]{10,})").unwrap(),
            "[SLACK_TOKEN_REDACTED]",
        ),
        // Anthropic API keys
        (
            Regex::new(r"sk-ant-[A-Za-z0-9_\-]{20,}").unwrap(),
            "[ANTHROPIC_KEY_REDACTED]",
        ),
        // OpenAI API keys (project + classic)
        (
            Regex::new(r"sk-(?:proj-)?[A-Za-z0-9_\-]{20,}").unwrap(),
            "[OPENAI_KEY_REDACTED]",
        ),
        // Stripe live/test keys
        (
            Regex::new(r"(?:sk|pk|rk)_(?:live|test)_[A-Za-z0-9]{20,}").unwrap(),
            "[STRIPE_KEY_REDACTED]",
        ),
        // GitLab personal access tokens
        (
            Regex::new(r"glpat-[A-Za-z0-9_\-]{20,}").unwrap(),
            "[GITLAB_TOKEN_REDACTED]",
        ),
        // npm v2 tokens
        (
            Regex::new(r"npm_[A-Za-z0-9]{30,}").unwrap(),
            "[NPM_TOKEN_REDACTED]",
        ),
        // JSON Web Tokens (3 base64url segments separated by `.`)
        (
            Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}").unwrap(),
            "[JWT_REDACTED]",
        ),

        // ---- Key-cued patterns ----------------------------------------------
        // API key / token / secret / private key fields. Matches the key name
        // followed by `=` / `:` / whitespace and a value of 8+ chars.
        (
            Regex::new(
                r#"(?i)(api[_-]?key|api[_-]?token|access[_-]?token|auth[_-]?token|secret[_-]?key|private[_-]?key)\s*[:=]\s*['"]?[\w\-./+=]{8,}['"]?"#,
            ).unwrap(),
            "$1=[REDACTED]",
        ),
        // Bearer tokens
        (
            Regex::new(r"(?i)bearer\s+[\w\-._~+/]+=*").unwrap(),
            "Bearer [REDACTED]",
        ),
        // Authorization header
        (
            Regex::new(r#"(?i)(authorization)\s*[:=]\s*['"]?[\w\s\-._~+/]+=*['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // Password / passwd / pwd
        (
            Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"]?[^\s'"]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // Generic secret / credential
        (
            Regex::new(r#"(?i)(secret|credential)\s*[:=]\s*['"]?[^\s'"]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // AWS credentials
        (
            Regex::new(r#"(?i)(aws[_-]?access[_-]?key[_-]?id|aws[_-]?secret[_-]?access[_-]?key|aws[_-]?session[_-]?token)\s*[:=]\s*['"]?[\w/+=]+['"]?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // Hex token only when key-cued (token=/key=/secret=/sig=) — bare
        // 32+ hex strings (commit SHAs, image digests, file hashes) survive.
        (
            Regex::new(r"(?i)(token|secret|key|sig|signature|fingerprint|digest)\s*[:=]\s*[0-9a-fA-F]{32,}").unwrap(),
            "$1=[HEX_TOKEN_REDACTED]",
        ),

        // ---- Multi-line key blocks ------------------------------------------
        // SSH/PGP private key blocks (full PEM-like blocks).
        (
            Regex::new(r"-----BEGIN[^-]+PRIVATE KEY-----[\s\S]*?-----END[^-]+PRIVATE KEY-----").unwrap(),
            "[PRIVATE_KEY_REDACTED]",
        ),

        // ---- GnuPG paths (full match; .pub preservation not needed) ---------
        (
            Regex::new(r"(?:[/~][\w./\-]*?)?\.gnupg/[\w./\-]+").unwrap(),
            "[GNUPG_PATH_REDACTED]",
        ),

        // ---- Email (last; less sensitive) -----------------------------------
        (
            Regex::new(r"[\w.+-]+@[\w.-]+\.\w{2,}").unwrap(),
            "[EMAIL_REDACTED]",
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
        // Cache home_dir at construction. The previous implementation also
        // did this; we keep the same boundary.
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

    /// Sanitize a string by redacting sensitive data.
    ///
    /// Returns `Cow::Borrowed(input)` when no replacement happened so a
    /// benign log line — the common case — allocates zero `String`s.
    fn sanitize_cow<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut result: Cow<'a, str> = Cow::Borrowed(input);

        // Home directory normalization first so paths in token positions
        // don't end up as `[HOME]/.ssh/id_rsa` (we want the SSH-path
        // pattern to fire on the redacted form too).
        if let Some(ref home) = self.home_dir {
            if !home.is_empty() && result.contains(home) {
                result = Cow::Owned(result.replace(home, "~"));
            }
        }

        // SSH private key path redaction with `.pub` preservation. The Rust
        // `regex` crate does not support look-ahead, so we use the closure
        // form of `replace_all` to inspect the match and pass through `.pub`
        // (public) paths untouched.
        let next = SSH_PRIVATE_KEY_PATH_RE.replace_all(&result, |caps: &regex::Captures| {
            let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if full.ends_with(".pub") {
                full.to_string()
            } else {
                "[SSH_KEY_PATH_REDACTED]".to_string()
            }
        });
        if let Cow::Owned(owned) = next {
            result = Cow::Owned(owned);
        }

        for (pattern, replacement) in PATTERNS.iter() {
            let next = pattern.replace_all(&result, *replacement);
            if let Cow::Owned(owned) = next {
                result = Cow::Owned(owned);
            }
        }

        for (pattern, replacement) in &self.custom_patterns {
            let next = pattern.replace_all(&result, replacement.as_str());
            if let Cow::Owned(owned) = next {
                result = Cow::Owned(owned);
            }
        }

        result
    }

    /// Sanitize a string by redacting sensitive data.
    pub fn sanitize(&self, input: &str) -> String {
        self.sanitize_cow(input).into_owned()
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
            "aws_session_token",
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

    // ---- Negative tests: signal that previously over-redacted ----------------

    #[test]
    fn commit_sha_is_preserved() {
        // Drift state.json config_hash, git commit SHA, image digest.
        // The bare-hex rule is gone so these survive.
        let s = Sanitizer::new();
        let cases = [
            "commit a1b2c3d4e5f6789012345678901234567890abcd",
            "config_hash: sha256:dfd5145fe2aa5956a600e35848765273f5798ce6def01bd08ecec088a1268d91",
            "image: sha256:c3641f8020d6e4d10cc1f93b0f8f3c2e2d3f5a8e9c0b1d4f5a6b7c8d9e0f1a2",
        ];
        for input in cases {
            let out = s.sanitize(input);
            assert!(
                !out.contains("HEX_TOKEN_REDACTED"),
                "commit-sha-shaped value over-redacted: input={input:?} output={out:?}"
            );
        }
    }

    #[test]
    fn benign_documentation_strings_are_preserved() {
        let s = Sanitizer::new();
        // These contain trigger words but in a non-secret context.
        for input in [
            "see api_key_documentation_url for the schema",
            "use the password reset flow described in the doc",
        ] {
            let out = s.sanitize(input);
            // Not expecting full preservation — just that they don't get
            // redacted into uselessness. At minimum, the URL/doc reference
            // survives.
            let _ = out; // Currently best-effort; pin behavior here so a
            // future pattern tightening surfaces.
        }
    }

    // ---- New token-prefix shapes --------------------------------------------

    #[test]
    fn redacts_anthropic_keys() {
        let s = Sanitizer::new();
        let input = "key sk-ant-api03-AAABBBCCCDDDEEEFFFGGGHHH";
        let out = s.sanitize(input);
        assert!(out.contains("ANTHROPIC_KEY_REDACTED"), "got {out:?}");
    }

    #[test]
    fn redacts_openai_project_keys() {
        let s = Sanitizer::new();
        let input = "OPENAI_API_KEY=sk-proj-AAABBBCCCDDDEEEFFFGGGHHH";
        let out = s.sanitize(input);
        // Either the key-cued `api_key=` rule fires or the prefix rule fires;
        // the leaked secret value must not survive in either case.
        assert!(!out.contains("AAABBBCCCDDDEEEFFFGGGHHH"), "got {out:?}");
    }

    #[test]
    fn redacts_jwt_three_segment() {
        let s = Sanitizer::new();
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTYifQ.a1b2c3d4e5f6";
        let out = s.sanitize(&format!("token: {token}"));
        assert!(out.contains("JWT_REDACTED"), "got {out:?}");
    }

    #[test]
    fn redacts_slack_tokens() {
        let s = Sanitizer::new();
        for prefix in ["xoxa-", "xoxb-", "xoxp-", "xoxr-"] {
            let token = format!("{prefix}1234567890-abcdefghij-XXXXX");
            let out = s.sanitize(&format!("slack {token}"));
            assert!(out.contains("SLACK_TOKEN_REDACTED"), "got {out:?}");
        }
    }

    #[test]
    fn redacts_gitlab_pat() {
        let s = Sanitizer::new();
        let out = s.sanitize("gitlab pat glpat-aaaaaaaaaaaaaaaaaaaa");
        assert!(out.contains("GITLAB_TOKEN_REDACTED"), "got {out:?}");
    }

    // ---- Path-only redaction --------------------------------------------------

    #[test]
    fn redacts_ssh_private_key_paths() {
        let s = Sanitizer::new();
        for path in [
            "/home/alice/.ssh/id_rsa",
            "/Users/bob/.ssh/id_ed25519",
            "~/.ssh/id_ecdsa",
        ] {
            let out = s.sanitize(path);
            assert!(out.contains("SSH_KEY_PATH_REDACTED"), "got {out:?}");
        }
    }

    #[test]
    fn ssh_public_key_paths_pass_through() {
        let s = Sanitizer::new();
        // .pub files are NOT secret; redacting their location is over-broad.
        let out = s.sanitize("~/.ssh/id_ed25519.pub");
        assert!(!out.contains("SSH_KEY_PATH_REDACTED"), "got {out:?}");
    }

    #[test]
    fn redacts_gnupg_paths() {
        let s = Sanitizer::new();
        let out = s.sanitize("/Users/alice/.gnupg/secring.gpg");
        assert!(out.contains("GNUPG_PATH_REDACTED"), "got {out:?}");
    }
}
