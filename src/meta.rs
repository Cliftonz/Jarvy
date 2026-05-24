//! Project-level identity constants.
//!
//! Single source of truth for the repo slug and canonical URLs that
//! would otherwise drift across docs, CI workflows, and source files.
//! When the project moves orgs / renames, only this file changes.
//!
//! Non-Rust consumers (docs, helm charts, GitHub workflows) still hold
//! their own copies — sweeping those is a separate cleanup. The goal of
//! this module is to stop the count of Rust-side hardcodes from growing.

#![allow(dead_code)] // Public API consumed across the crate.

/// `org/repo` slug used in GitHub URLs (issues, PRs, releases).
pub const REPO_SLUG: &str = "bearbinary/Jarvy";

/// Base repo URL — `https://github.com/<slug>`.
pub const REPO_URL: &str = "https://github.com/bearbinary/Jarvy";
