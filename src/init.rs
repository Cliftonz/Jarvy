use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use uuid::Uuid;

use crate::telemetry::TelemetryConfig;

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct CliConfig {
    pub settings: Settings,
    /// Telemetry configuration (OTLP endpoint, signals, etc.)
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Settings {
    /// Legacy telemetry switch (kept for backward compatibility)
    /// Use [telemetry] section for full configuration
    #[serde(default = "default_true")]
    pub telemetry: bool,
    #[serde(default)]
    pub fingerprint: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            telemetry: true,
            fingerprint: get_hwid_fingerprint().or_else(|| Some(Uuid::now_v7().to_string())),
        }
    }
}

pub(crate) fn initialize() -> CliConfig {
    // Test probe: allow tests to assert initialization ordering without side-effects
    if std::env::var("JARVY_INIT_PROBE").as_deref() == Ok("1") {
        eprintln!("TEST: initialize called");
    }
    // In test mode, avoid any filesystem side effects and just return defaults
    if std::env::var("JARVY_TEST_MODE").as_deref() == Ok("1") {
        return CliConfig::default();
    }

    // check jarvy config for the usr
    let home_dir = dirs::home_dir().expect("Failed to get home directory");

    // Create the .jarvy directory path
    let jarvy_dir = home_dir.join(".jarvy");

    // Define the path to the config.toml file
    let config_file_path = jarvy_dir.join("config.toml");

    // Create the .jarvy directory if it doesn't exist
    if !jarvy_dir.exists() {
        fs::create_dir(&jarvy_dir).expect("Unable to create jarvy config file");
        println!(
            r"
        Jarvy tool collects telemetry data to help us improve your experience.
        The data collected is anonymized and used solely for analytics purposes.
        If you wish to opt-out of telemetry collection, you can disable it by adding the following line to your configuration file located at ~/.jarvy/config.toml:
        [settings]
        telemetry = false

        Thank you for using Jarvy!
                "
        );

        // Write initial config
        let config = CliConfig {
            settings: Settings::default(),
            telemetry: TelemetryConfig::default(),
        };
        let toml = toml::to_string(&config).expect("serialize default config");
        let mut file = fs::File::create(&config_file_path).expect("Unable to create config file");
        file.write_all(toml.as_bytes())
            .expect("Unable to write content to config file");
    }

    // Read existing or just-created config.toml
    let config: CliConfig = {
        let config_content = fs::read_to_string(&config_file_path).unwrap_or_default();
        if config_content.trim().is_empty() {
            CliConfig::default()
        } else {
            toml::from_str(&config_content).unwrap_or_default()
        }
    };

    config
}

fn get_hwid_fingerprint() -> Option<String> {
    let mut builder = IdBuilder::new(Encryption::SHA256);

    // Add components for the fingerprint.
    builder
        .add_component(HWIDComponent::SystemID) // System UUID
        .add_component(HWIDComponent::CPUCores) // CPU core count
        .add_component(HWIDComponent::OSName) // Operating System name
        .add_component(HWIDComponent::DriveSerial); // Main disk serial

    // Build the ID with a custom key.
    const SALT: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15ac1e289f66085";
    // The key should be constant for your application to ensure consistency.
    builder.build(SALT).ok()
}
