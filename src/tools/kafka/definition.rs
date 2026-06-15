//! kafka - Apache Kafka distributed event streaming platform
//!
//! Apache Kafka itself — broker, ZooKeeper (legacy) / KRaft (modern)
//! controller, and the bundled CLI scripts (`kafka-console-producer`,
//! `kafka-console-consumer`, `kafka-topics`, etc.). Most real-world
//! Kafka work uses lighter CLIs (`kcat`, `rpk`, `kaf`) for everyday
//! pub/sub; install this when you need the full broker for local
//! development or want the official admin scripts.
//!
//! The `command:` is `kafka-topics` (one of the bundled scripts) —
//! `kafka` itself isn't a binary, the distribution exposes its
//! functionality through ~30 separate scripts. Picking
//! `kafka-topics` because it's the most-invoked one and is always
//! present.

use crate::define_tool;

define_tool!(KAFKA, {
    command: "kafka-topics",
    macos: { brew: "kafka" },
    linux: { uniform: "kafka" },
    windows: { winget: "Apache.Kafka" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kafka_registration_shape() {
        assert_eq!(KAFKA.command, "kafka-topics");
        let mac = KAFKA.macos.expect("kafka must support macOS");
        assert_eq!(mac.brew, Some("kafka"));
    }
}
