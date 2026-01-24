//! Git user identity handling
//!
//! Interactive prompting for git user identity and signing key detection.

use std::path::Path;
use std::process::Command;

use inquire::{Confirm, Text};

use super::config::{ConfigScope, ConfigValue, GitConfig};

/// Get current git config value
#[allow(dead_code)] // Public API for config inspection
pub fn get_current_config(key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--global", "--get", key])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Find default signing key path
#[allow(dead_code)] // Public API for interactive mode
pub fn find_default_signing_key() -> String {
    let home = dirs::home_dir().unwrap_or_default();

    // Check for common SSH keys in order of preference
    let keys = ["id_ed25519.pub", "id_ecdsa.pub", "id_rsa.pub"];

    for key in keys {
        let path = home.join(".ssh").join(key);
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }

    // Return default path even if it doesn't exist
    "~/.ssh/id_ed25519.pub".to_string()
}

/// Prompt user interactively for git identity
///
/// Returns an updated GitConfig with user-provided values, or the original
/// config if the user declines to update.
#[allow(dead_code)] // Public API for interactive mode
pub fn prompt_identity(existing: &GitConfig) -> Result<GitConfig, inquire::InquireError> {
    let mut config = existing.clone();

    // Check current configuration
    let current_name = get_current_config("user.name");
    let current_email = get_current_config("user.email");

    // If identity is already configured, ask if user wants to update
    if let (Some(name), Some(email)) = (&current_name, &current_email) {
        println!("\nGit identity already configured:");
        println!("  Name:  {name}");
        println!("  Email: {email}");

        let update = Confirm::new("Update Git identity?")
            .with_default(false)
            .prompt()?;

        if !update {
            return Ok(config);
        }
    }

    println!("\nGit Configuration");
    println!("=================\n");

    // Prompt for name
    let name = Text::new("Enter your name:")
        .with_default(current_name.as_deref().unwrap_or(""))
        .prompt()?;

    if !name.is_empty() {
        config.user_name = Some(ConfigValue::Plain(name));
    }

    // Prompt for email
    let email = Text::new("Enter your email:")
        .with_default(current_email.as_deref().unwrap_or(""))
        .prompt()?;

    if !email.is_empty() {
        config.user_email = Some(ConfigValue::Plain(email));
    }

    // Prompt for signing
    let enable_signing = Confirm::new("Enable commit signing?")
        .with_default(true)
        .prompt()?;

    if enable_signing {
        config.signing = true;

        let default_key = find_default_signing_key();
        let key = Text::new("Path to signing key:")
            .with_default(&default_key)
            .with_help_message("SSH public key (.pub) or GPG key ID")
            .prompt()?;

        if !key.is_empty() {
            // Validate key exists if it looks like a file path
            let expanded = shellexpand::tilde(&key);
            if Path::new(expanded.as_ref()).exists() {
                config.signing_key = Some(key);
            } else if !key.contains('/') && !key.contains('\\') {
                // Assume it's a GPG key ID
                config.signing_key = Some(key);
            } else {
                println!("  Warning: Signing key not found at {key}");
                let proceed = Confirm::new("Continue anyway?")
                    .with_default(false)
                    .prompt()?;

                if proceed {
                    config.signing_key = Some(key);
                } else {
                    config.signing = false;
                }
            }
        }
    }

    // Set scope to global by default for interactive mode
    config.scope = ConfigScope::Global;

    println!();
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_default_signing_key() {
        let key = find_default_signing_key();
        // Should return a path that ends with .pub or a default
        assert!(key.ends_with(".pub") || key.contains("id_"));
    }

    #[test]
    fn test_get_current_config_invalid() {
        // This should return None for a non-existent key
        let result = get_current_config("jarvy.nonexistent.key");
        assert!(result.is_none());
    }
}
