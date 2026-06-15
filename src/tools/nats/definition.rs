//! nats - NATS messaging system CLI
//!
//! The official NATS CLI for publishing, subscribing, managing
//! JetStream streams + consumers, replaying messages, and admin
//! operations against a NATS cluster. Pairs with `nats-server` for
//! local dev and `nsc` for credential management.

use crate::define_tool;

define_tool!(NATS, {
    command: "nats",
    macos: { brew: "nats-io/nats-tools/nats" },
    linux: { uniform: "natscli" },
    windows: { winget: "Synadia.NATSCli" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nats_registration_shape() {
        assert_eq!(NATS.command, "nats");
        let mac = NATS.macos.expect("nats must support macOS");
        assert_eq!(
            mac.brew,
            Some("nats-io/nats-tools/nats"),
            "macOS formula lives in the nats-io/nats-tools tap"
        );
        let win = NATS.windows.expect("nats must support Windows");
        assert_eq!(win.winget, Some("Synadia.NATSCli"));
    }
}
