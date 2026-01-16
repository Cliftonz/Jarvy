//! Fuzz target for arbitrary TOML input
//!
//! Generates structured TOML-like input to test config parsing edge cases.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzConfig {
    tool_name: String,
    version: String,
    use_sudo: Option<bool>,
    has_hooks: bool,
    hook_shell: Option<String>,
    hook_timeout: Option<u64>,
}

impl FuzzConfig {
    fn to_toml(&self) -> String {
        let mut toml = String::new();

        // Provisioner section
        toml.push_str("[provisioner]\n");

        // Sanitize tool name (only allow alphanumeric and hyphens)
        let safe_name: String = self
            .tool_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(50)
            .collect();

        let name = if safe_name.is_empty() {
            "tool".to_string()
        } else {
            safe_name
        };

        // Sanitize version
        let safe_version: String = self
            .version
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
            .take(20)
            .collect();

        let version = if safe_version.is_empty() {
            "latest".to_string()
        } else {
            safe_version
        };

        if let Some(sudo) = self.use_sudo {
            toml.push_str(&format!(
                "{} = {{ version = \"{}\", use_sudo = {} }}\n",
                name, version, sudo
            ));
        } else {
            toml.push_str(&format!("{} = \"{}\"\n", name, version));
        }

        // Hooks section
        if self.has_hooks {
            toml.push_str("\n[hooks.config]\n");

            if let Some(ref shell) = self.hook_shell {
                let safe_shell: String = shell
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(10)
                    .collect();
                if !safe_shell.is_empty() {
                    toml.push_str(&format!("shell = \"{}\"\n", safe_shell));
                }
            }

            if let Some(timeout) = self.hook_timeout {
                toml.push_str(&format!("timeout = {}\n", timeout % 86400)); // Cap at 1 day
            }
        }

        toml
    }
}

fuzz_target!(|config: FuzzConfig| {
    let toml_str = config.to_toml();

    // Parse the generated TOML
    let _ = toml::from_str::<toml::Value>(&toml_str);
});
