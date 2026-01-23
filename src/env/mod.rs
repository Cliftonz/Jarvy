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

// Public API exports - some may not be used internally but are part of the module's interface
#[allow(unused_imports)]
pub use dotenv::{DotenvConfig, DotenvError, generate_dotenv, preview_dotenv};
#[allow(unused_imports)]
pub use expand::{EnvContext, expand_value};
#[allow(unused_imports)]
pub use secrets::{SecretError, SecretsConfig, collect_secrets};
#[allow(unused_imports)]
pub use shell::{
    ShellConfig, ShellError, ShellType, detect_shell, get_rc_path, parse_shell, preview_shell_rc,
    update_shell_rc,
};
