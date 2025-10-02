use crate::tools;

/// Bootstrap a machine with base tools required across OSes.
/// Installs or ensures the presence of:
/// - git
/// - docker
/// - vscode
///
/// This function aims to be idempotent and will attempt installation only when
/// the tool is missing or version hint is not satisfied.
pub fn bootstrap() {
    // Register built-in tools
    tools::register_all();

    let tasks: &[(&str, &str)] = &[("git", ""), ("docker", ""), ("vscode", "")];

    for (name, hint) in tasks {
        match tools::add(name, hint) {
            Ok(_) => println!("✅ {} is installed or now installed", name),
            Err(e) => eprintln!("⛔ {} install/ensure failed: {}", name, e),
        }
    }
}
