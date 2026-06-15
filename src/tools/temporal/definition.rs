//! temporal - Temporal workflow CLI
//!
//! Temporal is a durable execution / workflow orchestration engine —
//! think "queue plus retries plus history plus replayable code." The
//! `temporal` CLI talks to a Temporal cluster (or `temporal server
//! start-dev` for local) and is the canonical way to start workflows,
//! query state, and replay history. Common pairing with NATS / Kafka
//! in event-driven microservices.

use crate::define_tool;

define_tool!(TEMPORAL, {
    command: "temporal",
    macos: { brew: "temporal" },
    linux: { uniform: "temporal" },
    windows: { winget: "Temporal.TemporalCLI" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_registration_shape() {
        assert_eq!(TEMPORAL.command, "temporal");
        let mac = TEMPORAL.macos.expect("temporal must support macOS");
        assert_eq!(mac.brew, Some("temporal"));
    }
}
