#[cfg(unix)]
use std::os::unix::prelude::CommandExt;
use std::path::Path;
use std::process::{Command, exit};
use std::{env, str};

use inquire::Select;

use crate::os_setup::set_up_os;
use crate::outputs::{error_message, installing_dependency, success_message};
use crate::provisioner::{
    check_and_install_git, install_docker, install_homebrew, start_docker_infra_with_config,
};
use crate::telemetry;

// Main function
pub fn setup() {
    const PLATFORM: &str = env::consts::OS;
    let start = telemetry::now();

    println!("Detecting Platform is: {}\n", PLATFORM);

    println!("Setting up defaults\n");
    set_up_os(PLATFORM);

    println!("\nInstalling Required Tools for {}\n", PLATFORM);

    check_hard_dependencies(PLATFORM);
    check_and_install_git(PLATFORM);
    install_docker();

    match PLATFORM {
        "macos" => {
            // install homebrew
            install_homebrew();
        }
        "linux" => {}
        "windows" => {}
        _ => {}
    }

    start_docker_infra_with_config(None);
    refresh_shell(PLATFORM);

    // Emit setup_complete with duration
    let summary = telemetry::SetupSummary {
        tools_requested: 0, // Legacy setup - minimal tracking
        tools_installed: 0,
        tools_skipped: 0,
        tools_failed: 0,
        hooks_run: 0,
        duration: start.elapsed(),
    };
    telemetry::setup_completed(&summary);
}

fn check_hard_dependencies(platform: &str) {
    // `platform` is `env::consts::OS` — lowercase. Same case-mismatch
    // bug as refresh_shell (was "macOS"); never fired on actual
    // macOS hosts. Hard-dep check is now actually reachable.
    match platform {
        "macos" => {
            let Some(output) = crate::tools::common::run_capture(
                "brew",
                &["--version"],
                "hard_dep_check",
                "Failed to run Homebrew check",
            ) else {
                return;
            };

            let brew_check = str::from_utf8(&output.stdout).unwrap_or("");

            if brew_check.is_empty() || output.status.code() != Some(0) {
                error_message("Homebrew");
                println!("⛔️ Homebrew is a hard dependency for this tool");

                installing_dependency("Homebrew");
                let Some(output) = crate::tools::common::run_capture(
                    "/bin/bash",
                    &[
                        "-c",
                        r#""$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#,
                    ],
                    "hard_dep_check",
                    "Failed to execute Homebrew install command",
                ) else {
                    return;
                };

                println!("{}", String::from_utf8_lossy(&output.stdout));
                success_message("Homebrew")
            }

            check_zsh();
        }
        "windows" => {}
        _ => {}
    }
}

fn check_zsh() {
    // Check if zsh is installed
    let Some(output) = crate::tools::common::run_capture(
        "zsh",
        &["--version"],
        "hard_dep_check",
        "Failed to check zsh",
    ) else {
        return;
    };

    // If zsh is not installed, don't go further.
    if output.status.code() != Some(0) {
        return;
    }

    // Zsh is installed, ask to install Oh My Zsh
    let user_choice = Select::new("Do you want to install Oh My Zsh?", vec!["Yes", "No"]).prompt();

    let Ok(response) = user_choice else {
        return;
    };

    // Check if user wants to install Oh My Zsh
    if response == "Yes" {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let ohmyzsh_dir = format!("{}/.oh-my-zsh", home.display());

        // Check if directory .oh-my-zsh exists in the home directory
        if !Path::new(&ohmyzsh_dir).exists() {
            // Download and install Oh My Zsh!
            if let Err(e) = Command::new("sh")
                .arg("-c")
                .arg("$(curl -fsSL https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh)")
                .status()
            {
                eprintln!("Failed to install Oh My Zsh: {e}");
                return;
            }

            // Check if Oh My Zsh! is installed successfully
            if !Path::new(&ohmyzsh_dir).exists() {
                println!("Error: Oh My Zsh!");
            } else {
                //success_message("Oh My Zsh!");
            }
        } else {
            println!("Oh My Zsh! is already installed.");
        }
    }
}

/// Single-quote a shell argument so it survives `sh -c "..."` unchanged.
/// Internal single quotes are escaped via the `'\''` idiom.
/// Rejects paths containing NUL bytes (sh cannot represent them).
pub(crate) fn shell_single_quote(s: &str) -> Option<String> {
    if s.contains('\0') {
        return None;
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    Some(out)
}

fn refresh_shell(platform: &str) {
    // `platform` is `env::consts::OS` — lowercase ("macos", "linux",
    // "windows"). The original arms used "macOS" (capital S) and
    // therefore never matched, falling through to the default arm
    // which printed a truncated "Unsupported sh" line. Match the
    // lowercase value the constant actually carries.
    match platform {
        "macos" => {
            let zprofile = env::var("ZPROFILE").unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
                format!("{home}/.zprofile")
            });

            if !Path::new(&zprofile).exists() {
                eprintln!(
                    "Warning: profile {} does not exist, skipping source",
                    zprofile
                );
                return;
            }

            let Some(quoted) = shell_single_quote(&zprofile) else {
                eprintln!("Error: ZPROFILE path contains a NUL byte: {}", zprofile);
                return;
            };

            let source_cmd = format!("source {}", quoted);
            let Some(output) = crate::tools::common::run_capture(
                "sh",
                &["-c", &source_cmd],
                "refresh_shell",
                "Failed to source shell profile",
            ) else {
                return;
            };

            if output.status.success() {
                let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
                #[cfg(unix)]
                {
                    let _ = Command::new(shell).exec();
                }
                #[cfg(not(unix))]
                {
                    let _ = Command::new(shell).status();
                }
            } else {
                eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                exit(1);
            }
        }
        "windows" => {
            let shell_profile = env::var("PROFILE").unwrap_or_else(|_| {
                let userprofile = env::var("USERPROFILE").unwrap_or_else(|_| "~".to_string());
                format!(
                    "{userprofile}\\Documents\\WindowsPowerShell\\Microsoft.PowerShell_profile.ps1"
                )
            });

            // PowerShell single-quote: doubled internal single quotes.
            if shell_profile.contains('\0') {
                eprintln!("Error: PROFILE path contains a NUL byte");
                return;
            }
            let ps_quoted = format!("'{}'", shell_profile.replace('\'', "''"));

            let dot_cmd = format!(". {}", ps_quoted);
            let Some(output) = crate::tools::common::run_capture(
                "powershell",
                &["-Command", &dot_cmd],
                "refresh_shell",
                "Failed to execute PowerShell command",
            ) else {
                return;
            };

            if output.status.success() {
                let _ = Command::new("powershell").status();
            } else {
                eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                exit(1);
            }
        }
        _ => {
            // Linux / FreeBSD / unknown — shell-refresh is a no-op.
            // The legacy "Unsupported sh" string printed here was a
            // typo (truncated "Unsupported shell") AND was reached
            // on macOS because the arm above checked the wrong case.
            // Silence both bugs.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::shell_single_quote;

    #[test]
    fn quotes_plain_path() {
        assert_eq!(
            shell_single_quote("/Users/zac/.zprofile"),
            Some("'/Users/zac/.zprofile'".to_string())
        );
    }

    #[test]
    fn neutralizes_semicolon_command_separator() {
        let injected = "/tmp/x;rm -rf $HOME";
        let q = shell_single_quote(injected).unwrap();
        assert_eq!(q, "'/tmp/x;rm -rf $HOME'");
        // After substitution into `source <q>`, sh sees one literal arg.
        assert!(!q.contains("\0"));
    }

    #[test]
    fn neutralizes_backtick_command_substitution() {
        let q = shell_single_quote("/tmp/`whoami`").unwrap();
        assert_eq!(q, "'/tmp/`whoami`'");
    }

    #[test]
    fn neutralizes_dollar_paren_substitution() {
        let q = shell_single_quote("/tmp/$(id)").unwrap();
        assert_eq!(q, "'/tmp/$(id)'");
    }

    #[test]
    fn neutralizes_pipe() {
        let q = shell_single_quote("/a|b").unwrap();
        assert_eq!(q, "'/a|b'");
    }

    #[test]
    fn neutralizes_glob_star() {
        // The original allowlist let `*` through and let `source /tmp/*.sh` glob.
        let q = shell_single_quote("/tmp/*.sh").unwrap();
        assert_eq!(q, "'/tmp/*.sh'");
    }

    #[test]
    fn neutralizes_newline() {
        let q = shell_single_quote("/a\nb").unwrap();
        assert_eq!(q, "'/a\nb'");
    }

    #[test]
    fn escapes_internal_single_quote() {
        // Caller's path with a single quote: foo'bar becomes 'foo'\''bar'
        let q = shell_single_quote("/a'b").unwrap();
        assert_eq!(q, "'/a'\\''b'");
    }

    #[test]
    fn rejects_nul_byte() {
        assert_eq!(shell_single_quote("/a\0b"), None);
    }

    #[test]
    fn quotes_unicode_paths() {
        let q = shell_single_quote("/Users/zac/プロファイル").unwrap();
        assert_eq!(q, "'/Users/zac/プロファイル'");
    }
}
