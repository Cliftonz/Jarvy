//! Team & Enterprise Collaboration Module (PRD-024)
//!
//! This module provides:
//! - Configuration inheritance with recursive extending
//! - Remote config caching with TTL
//! - Team config registry for shared sources
//! - Diamond dependency and circular reference detection

pub mod cache;
pub mod inheritance;
pub mod registry;

pub use cache::ConfigCache;
pub use inheritance::{InheritanceError, InheritanceResolver, ResolutionTrace};
pub use registry::{Registry, Source};

/// Extends configuration - supports single string or array of strings
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Extends {
    /// Single parent config
    Single(String),
    /// Multiple parent configs (processed left-to-right)
    Multiple(Vec<String>),
}

impl Extends {
    /// Get all parent config paths/URLs as a vector
    pub fn as_vec(&self) -> Vec<&str> {
        match self {
            Extends::Single(s) => vec![s.as_str()],
            Extends::Multiple(v) => v.iter().map(String::as_str).collect(),
        }
    }

    /// Check if this extends configuration is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Extends::Single(s) => s.is_empty(),
            Extends::Multiple(v) => v.is_empty(),
        }
    }
}

impl Default for Extends {
    fn default() -> Self {
        Extends::Multiple(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extends_single() {
        let extends = Extends::Single("https://example.com/base.toml".to_string());
        let vec = extends.as_vec();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], "https://example.com/base.toml");
        assert!(!extends.is_empty());
    }

    #[test]
    fn test_extends_multiple() {
        let extends = Extends::Multiple(vec![
            "https://example.com/base.toml".to_string(),
            "./local/override.toml".to_string(),
        ]);
        let vec = extends.as_vec();
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0], "https://example.com/base.toml");
        assert_eq!(vec[1], "./local/override.toml");
        assert!(!extends.is_empty());
    }

    #[test]
    fn test_extends_empty() {
        let extends = Extends::Multiple(Vec::new());
        assert!(extends.is_empty());

        let extends_single = Extends::Single(String::new());
        assert!(extends_single.is_empty());
    }

    #[test]
    fn test_extends_deserialize_single() {
        let toml_str = r#"extends = "https://example.com/base.toml""#;
        #[derive(serde::Deserialize)]
        struct Test {
            extends: Extends,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert!(matches!(test.extends, Extends::Single(_)));
    }

    #[test]
    fn test_extends_deserialize_multiple() {
        let toml_str = r#"extends = ["base.toml", "override.toml"]"#;
        #[derive(serde::Deserialize)]
        struct Test {
            extends: Extends,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert!(matches!(test.extends, Extends::Multiple(_)));
    }
}
