//! Git alias configuration
//!
//! Common git aliases and utilities for configuring them.

use std::collections::HashMap;

/// Standard set of recommended git aliases
#[allow(dead_code)] // Public API for alias recommendations
pub fn recommended_aliases() -> HashMap<String, String> {
    let mut aliases = HashMap::new();

    // Basic shortcuts
    aliases.insert("co".to_string(), "checkout".to_string());
    aliases.insert("br".to_string(), "branch".to_string());
    aliases.insert("ci".to_string(), "commit".to_string());
    aliases.insert("st".to_string(), "status".to_string());
    aliases.insert("cp".to_string(), "cherry-pick".to_string());

    // Useful compound commands
    aliases.insert(
        "lg".to_string(),
        "log --oneline --graph --decorate".to_string(),
    );
    aliases.insert("unstage".to_string(), "reset HEAD --".to_string());
    aliases.insert("last".to_string(), "log -1 HEAD".to_string());
    aliases.insert("undo".to_string(), "reset --soft HEAD~1".to_string());

    aliases
}

/// Format an alias for display
#[allow(dead_code)] // Public API for alias formatting
pub fn format_alias(name: &str, command: &str) -> String {
    if let Some(stripped) = command.strip_prefix('!') {
        format!("{name} = !{stripped}")
    } else {
        format!("{name} = {command}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommended_aliases() {
        let aliases = recommended_aliases();
        assert!(aliases.contains_key("co"));
        assert!(aliases.contains_key("br"));
        assert!(aliases.contains_key("ci"));
        assert!(aliases.contains_key("st"));
        assert_eq!(aliases.get("co"), Some(&"checkout".to_string()));
    }

    #[test]
    fn test_format_alias() {
        assert_eq!(format_alias("co", "checkout"), "co = checkout");
        assert_eq!(
            format_alias("sync", "!git fetch origin && git rebase origin/main"),
            "sync = !git fetch origin && git rebase origin/main"
        );
    }
}
