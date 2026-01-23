//! Self-updating functionality for Jarvy CLI
//!
//! This module provides automatic update checking and installation via:
//! - Multiple installation methods (Homebrew, Cargo, apt, dnf, winget, etc.)
//! - Background update checking with throttling
//! - Secure binary downloads with checksum verification
//! - Rollback support for failed updates

pub mod checker;
pub mod commands;
pub mod config;
pub mod installer;
pub mod method;
pub mod release;
pub mod rollback;
pub mod signature;

// Public API exports - some may not be used internally but are part of the module's interface
#[allow(unused_imports)]
pub use checker::{CURRENT_VERSION, CheckResult, UpdateChecker, UpdateState};
#[allow(unused_imports)]
pub use commands::{UpdateAction, run_update_command, show_update_notification_if_available};
#[allow(unused_imports)]
pub use config::{Channel, UpdateConfig};
#[allow(unused_imports)]
pub use installer::BinaryInstaller;
#[allow(unused_imports)]
pub use method::{InstallMethod, UpdateError};
#[allow(unused_imports)]
pub use release::{GitHubRelease, ReleaseAsset, ReleaseClient};
#[allow(unused_imports)]
pub use rollback::{RollbackInfo, RollbackManager, RollbackResult};
