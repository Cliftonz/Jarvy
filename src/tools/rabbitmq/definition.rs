//! rabbitmq-server - RabbitMQ AMQP/MQTT/STOMP broker
//!
//! The reference AMQP 0-9-1 broker (also speaks MQTT, STOMP, Stream
//! protocol). Ships with `rabbitmqctl` for cluster admin and
//! `rabbitmqadmin` for the HTTP API. The brew formula installs all
//! three; on Linux the distro packages do the same.

use crate::define_tool;

define_tool!(RABBITMQ, {
    command: "rabbitmq-server",
    macos: { brew: "rabbitmq" },
    linux: { uniform: "rabbitmq-server" },
    windows: { winget: "Pivotal.RabbitMQ" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rabbitmq_registration_shape() {
        assert_eq!(RABBITMQ.command, "rabbitmq-server");
        let mac = RABBITMQ.macos.expect("rabbitmq must support macOS");
        assert_eq!(mac.brew, Some("rabbitmq"));
        let win = RABBITMQ.windows.expect("rabbitmq must support Windows");
        assert_eq!(win.winget, Some("Pivotal.RabbitMQ"));
    }
}
