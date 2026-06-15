//! rpk - Redpanda / Kafka CLI
//!
//! `rpk` is Redpanda's Go-based CLI; it speaks the Kafka wire
//! protocol so it works equally well against Redpanda, Apache
//! Kafka, MSK, Confluent Cloud, and others. Higher-level UX than
//! the official Kafka shell scripts — `rpk topic create`,
//! `rpk cluster info`, `rpk produce`, `rpk consume`, etc. Pair with
//! `kcat` for low-level binary streaming.

use crate::define_tool;

define_tool!(RPK, {
    command: "rpk",
    macos: { brew: "redpanda-data/tap/redpanda" },
    linux: { uniform: "redpanda" },
    windows: { winget: "Redpanda.RPK" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpk_registration_shape() {
        assert_eq!(RPK.command, "rpk");
        let mac = RPK.macos.expect("rpk must support macOS");
        assert_eq!(
            mac.brew,
            Some("redpanda-data/tap/redpanda"),
            "macOS formula lives in the redpanda-data/tap tap"
        );
    }
}
