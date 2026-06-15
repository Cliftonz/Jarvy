//! nats-server - NATS messaging broker
//!
//! The NATS broker itself. Single-binary cloud-native messaging
//! system (Core NATS, JetStream persistence, Key/Value, Object Store).
//! Use this for local development; production typically runs the
//! same binary in a cluster.

use crate::define_tool;

define_tool!(NATS_SERVER, {
    command: "nats-server",
    macos: { brew: "nats-server" },
    linux: { uniform: "nats-server" },
    windows: { winget: "Synadia.NATSServer" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nats_server_registration_shape() {
        assert_eq!(NATS_SERVER.command, "nats-server");
        let mac = NATS_SERVER.macos.expect("nats-server must support macOS");
        assert_eq!(mac.brew, Some("nats-server"));
        let win = NATS_SERVER
            .windows
            .expect("nats-server must support Windows");
        assert_eq!(win.winget, Some("Synadia.NATSServer"));
    }
}
