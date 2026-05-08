//! Shared HTTP / network primitives. Currently a single-module wrapper
//! around `ureq::Agent` with timeouts, but the right home for any future
//! cross-cutting network helpers (URL allowlist, redirect callback, etc.).

pub mod agent;

#[allow(unused_imports)]
pub use agent::user_agent;
pub use agent::{USER_AGENT, agent};
