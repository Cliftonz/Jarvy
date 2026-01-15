//! Environment variables management module
//!
//! This module provides functionality for:
//! - Variable expansion ($HOME, $PWD, $USER, etc.)
//! - .env file generation
//! - Shell rc file modification
//! - Secret prompting with hidden input

mod dotenv;
mod expand;
mod secrets;
mod shell;

pub use dotenv::{generate_dotenv, preview_dotenv, DotenvConfig, DotenvError};
pub use expand::{expand_value, EnvContext};
pub use secrets::{collect_secrets, SecretsConfig, SecretError};
pub use shell::{
    detect_shell, get_rc_path, parse_shell, preview_shell_rc, update_shell_rc, ShellConfig,
    ShellError, ShellType,
};
