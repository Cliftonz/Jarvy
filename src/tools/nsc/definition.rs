//! nsc - NATS account/credential management CLI
//!
//! Tool for managing NATS decentralized auth — operators, accounts,
//! users, signing keys, and credential files. Required when running
//! NATS with the JWT-based auth system (the default for managed NATS
//! cloud and most production clusters).

use crate::define_tool;

define_tool!(NSC, {
    command: "nsc",
    macos: { brew: "nats-io/nats-tools/nsc" },
    linux: { uniform: "nsc" },
    windows: { winget: "Synadia.NSC" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nsc_registration_shape() {
        assert_eq!(NSC.command, "nsc");
        let mac = NSC.macos.expect("nsc must support macOS");
        assert_eq!(
            mac.brew,
            Some("nats-io/nats-tools/nsc"),
            "macOS formula lives in the nats-io/nats-tools tap"
        );
    }
}
