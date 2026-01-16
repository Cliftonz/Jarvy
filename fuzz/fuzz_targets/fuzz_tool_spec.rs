//! Fuzz target for tool specification parsing
//!
//! Tests parsing of tool specifications in both simple and detailed formats.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum FuzzToolSpec {
    /// Simple version string: tool = "1.0.0"
    Simple(String),
    /// Detailed spec: tool = { version = "1.0.0", version_manager = true }
    Detailed {
        version: String,
        version_manager: Option<bool>,
        use_sudo: Option<bool>,
    },
}

impl FuzzToolSpec {
    fn to_toml_value(&self) -> String {
        match self {
            FuzzToolSpec::Simple(v) => {
                let safe: String = v
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                    .take(50)
                    .collect();
                format!("\"{}\"", if safe.is_empty() { "latest" } else { &safe })
            }
            FuzzToolSpec::Detailed {
                version,
                version_manager,
                use_sudo,
            } => {
                let safe_version: String = version
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                    .take(50)
                    .collect();
                let v = if safe_version.is_empty() {
                    "latest"
                } else {
                    &safe_version
                };

                let mut parts = vec![format!("version = \"{}\"", v)];

                if let Some(vm) = version_manager {
                    parts.push(format!("version_manager = {}", vm));
                }
                if let Some(sudo) = use_sudo {
                    parts.push(format!("use_sudo = {}", sudo));
                }

                format!("{{ {} }}", parts.join(", "))
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
struct FuzzToolConfig {
    tools: Vec<(String, FuzzToolSpec)>,
}

impl FuzzToolConfig {
    fn to_toml(&self) -> String {
        let mut toml = String::from("[provisioner]\n");

        for (i, (name, spec)) in self.tools.iter().take(20).enumerate() {
            let safe_name: String = name
                .chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .take(30)
                .collect();

            let tool_name = if safe_name.is_empty()
                || !safe_name.chars().next().unwrap().is_ascii_lowercase()
            {
                format!("tool{}", i)
            } else {
                safe_name
            };

            toml.push_str(&format!("{} = {}\n", tool_name, spec.to_toml_value()));
        }

        toml
    }
}

fuzz_target!(|config: FuzzToolConfig| {
    if config.tools.is_empty() {
        return;
    }

    let toml_str = config.to_toml();

    // Parse the generated TOML
    let _ = toml::from_str::<toml::Value>(&toml_str);
});
