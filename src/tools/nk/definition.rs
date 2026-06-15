//! nk - NATS key generation / signing utility
//!
//! Low-level companion to `nsc` for generating NATS-format nkey
//! seeds + public keys (account/user/server/cluster identities) and
//! signing nonces during connection handshakes. Most users only need
//! it for advanced auth flows or CI signing pipelines; everyday work
//! goes through `nsc`.

use crate::define_tool;

define_tool!(NK, {
    command: "nk",
    macos: { brew: "nats-io/nats-tools/nk" },
    linux: { uniform: "nk" },
    windows: { winget: "Synadia.NK" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nk_registration_shape() {
        assert_eq!(NK.command, "nk");
        let mac = NK.macos.expect("nk must support macOS");
        assert_eq!(
            mac.brew,
            Some("nats-io/nats-tools/nk"),
            "macOS formula lives in the nats-io/nats-tools tap"
        );
    }
}
