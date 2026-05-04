use std::process::{Command, Output};
use std::str;

use inquire::Select;

pub(crate) fn handle_output(output: &Output) {
    if !output.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn get_cpu() -> String {
    let output = match Command::new("uname").arg("-m").output() {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Failed to fetch CPU info: {e}");
            return String::new();
        }
    };

    if output.status.success() {
        let s = str::from_utf8(&output.stdout).unwrap_or_default();
        s.trim().to_string()
    } else {
        eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
        String::new()
    }
}

fn set_macosx_generics() {
    println!("Set MacOSx system configurations");

    let output = match std::process::Command::new("defaults")
        .arg("write")
        .arg("com.apple.finder")
        .arg("AppleShowAllFiles")
        .arg("YES")
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Failed to execute defaults command: {e}");
            return;
        }
    };

    if !output.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

pub fn set_up_os(platform: &str) {
    match platform {
        "macos" => {
            println!("Setting macos to show all file types.");

            let output = match std::process::Command::new("defaults")
                .arg("write")
                .arg("com.apple.finder")
                .arg("AppleShowAllFiles")
                .arg("YES")
                .output()
            {
                Ok(output) => output,
                Err(e) => {
                    eprintln!("Failed to execute defaults command: {e}");
                    return;
                }
            };

            if !output.status.success() {
                eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
            }

            if get_cpu() != "arm64" {
                println!("Setting up Rosetta for arm64 emulation");

                match Command::new("softwareupdate")
                    .args(["--install-rosetta"])
                    .output()
                {
                    Ok(output) => handle_output(&output),
                    Err(e) => eprintln!("Failed to start Rosetta installation: {e}"),
                }
            }

            set_macosx_generics();

            let xcode_check = match Command::new("/usr/bin/xcode-select").args(["-p"]).output() {
                Ok(output) => output,
                Err(e) => {
                    eprintln!("Failed to check Xcode installation: {e}");
                    return;
                }
            };

            if xcode_check.status.success() {
                let update_xcode_prompt = Select::new(
                    "Xcode is already installed, do you want to update it?",
                    vec!["Yes", "No"],
                )
                .prompt();

                println!("\n");

                match update_xcode_prompt {
                    Ok(answer) => match answer {
                        "Yes" => match Command::new("softwareupdate").arg("-ia").spawn() {
                            Ok(mut child) => {
                                if let Err(e) = child.wait() {
                                    eprintln!("Failed to wait on software update: {e}");
                                }
                            }
                            Err(e) => eprintln!("Failed to start software update: {e}"),
                        },
                        "No" => {
                            println!("Xcode will not be updated.");
                        }
                        _ => unreachable!(),
                    },
                    Err(_) => {
                        println!("Could not read your response.");
                    }
                }
            } else {
                println!("Installing Xcode...");
                match Command::new("xcode-select").args(["--install"]).spawn() {
                    Ok(mut child) => {
                        if let Err(e) = child.wait() {
                            eprintln!("Failed to wait on Xcode installation: {e}");
                        }
                    }
                    Err(e) => eprintln!("Failed to start Xcode installation: {e}"),
                }
            }
        }
        "Linux" => {
            println!("Nothing to configure");
        }
        "Windows" => {
            println!("Set Windows system configurations");

            let output = match std::process::Command::new("powershell")
                .arg("/c")
                .arg("Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced' -Name 'Hidden' -Value 1")
                .output()
            {
                Ok(output) => output,
                Err(e) => {
                    eprintln!("Failed to execute powershell command: {e}");
                    return;
                }
            };

            if !output.status.success() {
                eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        _ => println!("Unsupported platform"),
    }
    println!("\n");
}
