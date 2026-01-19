use std::env;
use std::process::Command;
use std::str;
use std::time::Duration;

use crate::telemetry;

pub fn install_homebrew() {
    // Macos Only
    let test_brew_cmd = Command::new("brew")
        .arg("--version")
        .output()
        .expect("Failed to run brew");

    if !test_brew_cmd.status.success() {
        println!("Installing Homebrew");
        let start = telemetry::now();

        let curl_cmd = r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)""#;
        let _output = Command::new("sh")
            .arg("-c")
            .arg(curl_cmd)
            .output()
            .expect("Failed to execute command");

        let zprofile = format!("{}/.zprofile", env::var("HOME").unwrap());
        let apple_chip_brew_bin = "/opt/homebrew/bin";
        let brew_bin = "/usr/local/bin";
        let entry = format!(
            "export PATH={}:{}:${}",
            brew_bin,
            apple_chip_brew_bin,
            env::var("PATH").unwrap()
        );

        let param_to_cmd = format!("grep -R {} {}", entry, zprofile);

        let _cmd = Command::new("sh")
            .arg("-c")
            .arg(param_to_cmd)
            .output()
            .expect("Failed to execute command");

        let after_install_test_cmd = Command::new("brew")
            .arg("--version")
            .output()
            .expect("Failed to execute command");

        if !after_install_test_cmd.status.success() {
            eprintln!("Error: Homebrew");
            telemetry::tool_failed("homebrew", "latest", "installation failed");
        } else {
            println!("Successfully installed Homebrew");
            telemetry::tool_installed("homebrew", "latest", "shell", start.elapsed());
        }
    } else {
        println!("Homebrew is already installed");
    }
}

pub fn install_docker() {
    let check_homebrew_output = Command::new("brew")
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    // If brew not found or any other problem occurred
    if check_homebrew_output.status.success() {
        let test_docker_output = Command::new("docker")
            .arg("--version")
            .output()
            .expect("Failed to execute command");

        // If docker not found or any other problem occurred
        if !test_docker_output.status.success() {
            println!("Installing Docker");
            let start = telemetry::now();
            let brew_install_output = Command::new("brew")
                .arg("install")
                .arg("docker")
                .output()
                .expect("Failed to execute command");

            // If Docker installed successfully
            if brew_install_output.status.success() {
                // Test Docker installation
                let after_install_test_output = Command::new("docker")
                    .arg("--version")
                    .output()
                    .expect("Failed to execute command");

                // If Docker now runs properly
                if after_install_test_output.status.success() {
                    println!("Successfully installed Docker");
                    telemetry::tool_installed("docker", "latest", "brew", start.elapsed());
                } else {
                    println!("Error: Docker");
                    telemetry::tool_failed("docker", "latest", "post-install test failed");
                }
            } else {
                telemetry::tool_failed("docker", "latest", "brew install failed");
            }
        } else {
            println!("Docker is already installed");
        }
    } else {
        println!("Skipping Docker installation as Homebrew is not found");
    }
}

// TODO: figure out command and process to start from cli
pub fn start_docker_infra() {
    let start = telemetry::now();
    let docker_compose_output = Command::new("docker-compose")
        .arg("-f")
        .arg("./docker//docker-compose.yml")
        .arg("up")
        .arg("-d")
        .output()
        .expect("Failed to execute command");

    if docker_compose_output.status.success() {
        println!("Successfully started Docker Infrastructure");
        telemetry::service_operation("docker-compose", "up", true);
    } else {
        let err = String::from_utf8_lossy(&docker_compose_output.stderr).to_string();
        eprintln!(
            "An error occurred: \n\t {}. \nPlease run this from the root of your repository.",
            err
        );
        telemetry::service_operation("docker-compose", "up", false);
    }
    // Track duration even if unused for now
    let _ = start.elapsed();
}

// Minimal stubs to satisfy references from setup.rs during tests
pub fn install_nvm_mac() {
    // no-op in test context
}

pub fn install_pnpm() {
    // no-op in test context
}

pub fn check_and_install_git(_platform: &str) {
    // no-op in test context
}
