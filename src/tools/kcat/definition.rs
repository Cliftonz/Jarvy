//! kcat - Apache Kafka swiss-army CLI (was `kafkacat`)
//!
//! `kcat` is the netcat-equivalent for Kafka — produce, consume,
//! list topics, query metadata, debug consumer groups. Tiny single
//! binary, the daily-driver Kafka CLI for most operators. Renamed
//! from `kafkacat` upstream in 2021; the brew formula now ships as
//! `kcat`. Older Debian / Ubuntu releases still ship `kafkacat` as
//! the package name — the uniform name here uses the new one.

use crate::define_tool;

define_tool!(KCAT, {
    command: "kcat",
    macos: { brew: "kcat" },
    linux: { uniform: "kcat" },
    windows: { winget: "edenhill.kcat" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kcat_registration_shape() {
        assert_eq!(KCAT.command, "kcat");
        let mac = KCAT.macos.expect("kcat must support macOS");
        assert_eq!(
            mac.brew,
            Some("kcat"),
            "post-rename formula is `kcat`, not `kafkacat`"
        );
    }
}
