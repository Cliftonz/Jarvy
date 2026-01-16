//! CLI command implementations for PRD-016 Developer Experience Commands
//!
//! This module contains implementations for:
//! - `jarvy doctor` - Environment diagnostics
//! - `jarvy diff` - Preview changes before setup
//! - `jarvy export` - Generate jarvy.toml from installed tools
//! - `jarvy upgrade` - Upgrade tools to latest versions
//! - `jarvy search` - Search available tools
//! - `jarvy validate` - Validate configuration files
//! - `jarvy completions` - Generate shell completions

pub mod completions;
pub mod diff;
pub mod doctor;
pub mod export;
pub mod search;
pub mod upgrade;
pub mod validate;

pub use completions::*;
pub use diff::*;
pub use doctor::*;
pub use export::*;
pub use search::*;
pub use upgrade::*;
pub use validate::*;
