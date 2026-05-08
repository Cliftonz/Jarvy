//! Shared `ureq::Agent` with sane timeouts and a cached User-Agent header.
//!
//! Previous code constructed a fresh `ureq::Agent::new_with_defaults()` per
//! HTTP call (`remote::fetch_remote_config`, `update::installer::download_*`,
//! `update::release::*`, `team::*`). Each fresh agent does its own TLS
//! handshake; for `jarvy update` that means three handshakes against
//! `objects.githubusercontent.com` for archive + checksums + signature
//! companions — 100–400 ms of avoidable latency per update.
//!
//! Sharing a single agent also lets us pin a sensible global timeout. The
//! previous default had no overall budget, so a slow-loris attacker on an
//! allowlisted host (compromised raw.githubusercontent.com mirror, MitM
//! through a corp proxy) could hold `jarvy update` open indefinitely while
//! the staging file existed in `~/.jarvy/staging/` un-verified.
//!
//! Use `agent()` for any GET that should respect Jarvy's network policy.

use std::sync::LazyLock;
use std::time::Duration;

/// Shared agent for all outbound HTTP. Built lazily on first use.
///
/// `max_redirects(0)` disables ureq's default 10-hop auto-follow.
/// Without this disable, `remote::validated_get` and
/// `fetch_remote_config` only check the URL host once — a 302 to a
/// non-allowlisted host is followed silently, bypassing the entire
/// allowlist. Round-2 security P1: callers that need to follow a
/// redirect must call `validated_get` again with the resolved URL,
/// re-running the policy check on the new host.
static SHARED_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    let config = ureq::Agent::config_builder()
        // 60s overall budget, 30s for read/write; the install-binary path
        // wraps the TLS handshake + body transfer end-to-end inside this.
        .timeout_global(Some(Duration::from_secs(60)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(30)))
        .timeout_send_request(Some(Duration::from_secs(30)))
        .timeout_send_body(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .build();
    ureq::Agent::new_with_config(config)
});

/// Returns the process-wide `ureq::Agent`. Callers should attach their own
/// `User-Agent` header per request.
pub fn agent() -> &'static ureq::Agent {
    &SHARED_AGENT
}

/// Standard `User-Agent` string for jarvy outbound requests.
///
/// `pub const &str` so callers don't pay an allocation per request.
pub const USER_AGENT: &str = concat!("jarvy/", env!("CARGO_PKG_VERSION"));

/// Standard `User-Agent` string for jarvy outbound requests. Kept as a
/// function for backward compatibility with sites that take `&str`.
#[allow(dead_code)] // Callers can prefer `USER_AGENT` const.
pub fn user_agent() -> &'static str {
    USER_AGENT
}
