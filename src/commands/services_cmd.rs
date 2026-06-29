//! Services command handler - manage project services (docker-compose, tilt)

use crate::ci;
use crate::cli::ServicesAction;
use crate::config::Config;
use crate::services;

/// Run the services command
pub fn run_services(action: &ServicesAction, file: &str) -> i32 {
    let output_format = match action {
        ServicesAction::Start { output_format, .. }
        | ServicesAction::Stop { output_format, .. }
        | ServicesAction::Status { output_format, .. }
        | ServicesAction::Restart { output_format, .. } => output_format.as_str(),
    };
    let is_json = output_format == "json";

    let config = Config::new(file);
    let services_config = config.services.clone();

    if !services_config.enabled {
        if is_json {
            println!(
                "{}",
                serde_json::json!({"status": "disabled", "message": "services not enabled in jarvy.toml"})
            );
        } else {
            eprintln!("Services are not enabled in the configuration.");
            eprintln!("Add [services] enabled = true to your jarvy.toml");
        }
        return 0;
    }

    let _is_ci = ci::detect().is_some();

    let working_dir = std::path::Path::new(file)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    let backend_result = services::detect_backend_with_config(
        working_dir,
        services_config.compose_file.as_deref(),
        services_config.tilt_file.as_deref(),
    );

    let (backend, config_path) = match backend_result {
        Some((b, p)) => (b, p),
        None => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "no_backend", "supported": ["docker-compose.yml", "compose.yml", "Tiltfile"]})
                );
            } else {
                eprintln!("No service configuration found.");
                eprintln!("Supported: docker-compose.yml, compose.yml, Tiltfile");
            }
            return 0;
        }
    };

    let backend_impl = services::get_backend(backend);

    if !backend_impl.is_installed() {
        if is_json {
            println!(
                "{}",
                serde_json::json!({
                    "status": "backend_not_installed",
                    "backend": backend.to_string(),
                    "remedy": "jarvy setup",
                })
            );
        } else {
            println!("{} is not installed.", backend);
            println!("Install it with: jarvy setup");
        }
        return 0;
    }

    match action {
        ServicesAction::Start { foreground, .. } => {
            let detach = !foreground;
            if !is_json {
                println!("Starting {} services...", backend);
            }
            match backend_impl.start(&config_path, detach) {
                Ok(result) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "action": "start",
                                "backend": backend.to_string(),
                                "status": "ok",
                                "message": result.message,
                            })
                        );
                    } else {
                        println!("{}", result.message);
                    }
                }
                Err(e) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({"action": "start", "status": "error", "error": e.to_string()})
                        );
                    } else {
                        eprintln!("Failed to start services: {}", e);
                    }
                    return 1;
                }
            }
        }
        ServicesAction::Stop { .. } => {
            if !is_json {
                println!("Stopping {} services...", backend);
            }
            match backend_impl.stop(&config_path) {
                Ok(result) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "action": "stop",
                                "backend": backend.to_string(),
                                "status": "ok",
                                "message": result.message,
                            })
                        );
                    } else {
                        println!("{}", result.message);
                    }
                }
                Err(e) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({"action": "stop", "status": "error", "error": e.to_string()})
                        );
                    } else {
                        eprintln!("Failed to stop services: {}", e);
                    }
                    return 1;
                }
            }
        }
        ServicesAction::Status { .. } => match backend_impl.status(&config_path) {
            Ok(status) => {
                if is_json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "action": "status",
                            "backend": status.backend.to_string(),
                            "installed": status.installed,
                            "running": status.running,
                            "details": status.details,
                        })
                    );
                } else {
                    println!("Service Backend: {}", status.backend);
                    println!("Installed: {}", if status.installed { "Yes" } else { "No" });
                    println!("Running: {}", if status.running { "Yes" } else { "No" });
                    if !status.details.is_empty() {
                        println!("\nDetails:\n{}", status.details);
                    }
                }
            }
            Err(e) => {
                if is_json {
                    println!(
                        "{}",
                        serde_json::json!({"action": "status", "status": "error", "error": e.to_string()})
                    );
                } else {
                    eprintln!("Failed to get service status: {}", e);
                }
                return 1;
            }
        },
        ServicesAction::Restart { foreground, .. } => {
            let detach = !foreground;
            if !is_json {
                println!("Restarting {} services...", backend);
            }
            match backend_impl.restart(&config_path, detach) {
                Ok(result) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "action": "restart",
                                "backend": backend.to_string(),
                                "status": "ok",
                                "message": result.message,
                            })
                        );
                    } else {
                        println!("{}", result.message);
                    }
                }
                Err(e) => {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({"action": "restart", "status": "error", "error": e.to_string()})
                        );
                    } else {
                        eprintln!("Failed to restart services: {}", e);
                    }
                    return 1;
                }
            }
        }
    }
    0
}
