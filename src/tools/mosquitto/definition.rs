//! mosquitto - Eclipse MQTT broker + pub/sub clients
//!
//! Reference MQTT 3.1 / 3.1.1 / 5.0 broker from the Eclipse
//! Foundation. The brew formula installs the broker (`mosquitto`)
//! plus the `mosquitto_pub` / `mosquitto_sub` CLIs. Common in IoT
//! and edge-compute stacks where NATS / Kafka are too heavy.

use crate::define_tool;

define_tool!(MOSQUITTO, {
    command: "mosquitto",
    macos: { brew: "mosquitto" },
    linux: { uniform: "mosquitto" },
    windows: { winget: "EclipseFoundation.Mosquitto" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mosquitto_registration_shape() {
        assert_eq!(MOSQUITTO.command, "mosquitto");
        let mac = MOSQUITTO.macos.expect("mosquitto must support macOS");
        assert_eq!(mac.brew, Some("mosquitto"));
        let win = MOSQUITTO.windows.expect("mosquitto must support Windows");
        assert_eq!(win.winget, Some("EclipseFoundation.Mosquitto"));
    }
}
