//! Git configuration automation module
//!
//! This module provides functionality to configure Git settings including:
//! - User identity (name, email)
//! - Commit signing (SSH/GPG keys)
//! - Default settings (branch, pull strategy, editor)
//! - Git aliases
//! - Credential helpers
//!
//! Configuration is read from the `[git]` section of `jarvy.toml`.

mod aliases;
mod config;
mod identity;
mod setup;
mod signing;

pub use config::GitConfig;

// Public API types for interactive mode and advanced usage
#[allow(unused_imports)]
pub use config::{AutoCrlf, ConfigScope, ConfigValue, SigningFormat};
#[allow(unused_imports)]
pub use setup::GitError;
pub use setup::GitSetup;

// These are public API functions for interactive mode and validation
#[allow(unused_imports)]
pub use identity::{find_default_signing_key, get_current_config, prompt_identity};
