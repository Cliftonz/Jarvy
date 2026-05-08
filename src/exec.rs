//! Top-level subprocess primitives.
//!
//! Re-exports the canonical helpers from `crate::tools::common` so callers
//! outside the `tools/` subtree don't have to reach into a tool-specific
//! module to spawn `git`, `docker compose`, `cargo`, or any other
//! subprocess.
//!
//! Why a re-export instead of a move? `tools::common::run` is already
//! used by 100+ tool definitions; renaming the module would touch every
//! one of them. This module is the seam future cross-subsystem code
//! should reach for; existing `tools/*` keeps using `crate::tools::common`.
//!
//! Maintainability review F-7 flagged that `packages/`, `git/`,
//! `services/`, and `update/method/` were each rolling their own
//! `Command::new(...)` patterns despite this primitive being available.
//! Migrate those one subsystem at a time as each is touched.

#![allow(dead_code, unused_imports)] // Public seam; callers migrate incrementally.

pub use crate::tools::common::{
    InstallError, has, require, require_any, run, run_capture, run_maybe_sudo,
    run_maybe_sudo_with_network, run_with_network,
};
