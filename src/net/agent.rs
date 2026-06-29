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

/// Dedicated agent for the GitHub Releases API client (`update::release`).
///
/// Differs from `SHARED_AGENT` in exactly one knob: `max_redirects(3)`.
/// GitHub's REST API permanently redirects every endpoint of a renamed
/// repository to `api.github.com/repositories/<id>/...` (same host). The
/// shared agent's `max_redirects(0)` policy — correct for user-supplied
/// remote-config fetches — turns those 301s into a JSON parse failure,
/// which the install path swallows as "up to date" under sandbox
/// auto-disable. Effect: every binary built before a future repo rename
/// silently stops self-updating until the user runs `cargo install
/// jarvy --force` (or equivalent) out-of-band.
///
/// Limited to 3 hops so a misconfigured loop on api.github.com still
/// terminates quickly. The host allowlist policy that `SHARED_AGENT`
/// protects does not apply here: `update::release` only ever calls the
/// hardcoded `api.github.com/repos/{owner}/{repo}/...` paths.
static GITHUB_API_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(30)))
        .timeout_send_request(Some(Duration::from_secs(30)))
        .timeout_send_body(Some(Duration::from_secs(30)))
        .max_redirects(3)
        .build();
    ureq::Agent::new_with_config(config)
});

/// Returns the process-wide `ureq::Agent`. Callers should attach their own
/// `User-Agent` header per request.
pub fn agent() -> &'static ureq::Agent {
    &SHARED_AGENT
}

/// Returns the dedicated agent for GitHub Releases API calls.
///
/// Use this *only* for hardcoded `api.github.com/repos/{owner}/{repo}/...`
/// URLs in `update::release`. Following redirects there is safe and
/// future-proofs against another `bearbinary/jarvy` → `Cliftonz/jarvy`
/// style rename bricking the auto-updater for already-installed binaries.
pub fn github_api_agent() -> &'static ureq::Agent {
    &GITHUB_API_AGENT
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
