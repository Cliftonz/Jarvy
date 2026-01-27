//! Handler for the `jarvy logs` command
//!
//! View and manage log files.

use crate::cli::LogsAction;
use crate::logging;

/// Handle logs command dispatch
pub fn run_logs_command(action: LogsAction) -> i32 {
    match action {
        LogsAction::View {
            lines,
            level,
            grep,
            output_format,
        } => handle_logs_view(lines, level, grep, &output_format),
        LogsAction::Stats {} => handle_logs_stats(),
        LogsAction::Clean { all, dry_run } => handle_logs_clean(all, dry_run),
        LogsAction::Config {} => handle_logs_config(),
    }
}

/// View recent log entries
fn handle_logs_view(
    lines: usize,
    level_filter: Option<String>,
    grep_filter: Option<String>,
    output_format: &str,
) -> i32 {
    match logging::read_recent_logs(lines) {
        Ok(logs) => {
            if logs.is_empty() {
                println!("No log entries found.");
                return 0;
            }

            // Apply filters
            let filtered: Vec<&String> = logs
                .iter()
                .filter(|line| {
                    // Level filter
                    if let Some(ref level) = level_filter {
                        let level_upper = level.to_uppercase();
                        let has_level = line.contains(&format!("\"level\":\"{}\"", level_upper))
                            || line.contains(&format!(" {} ", level_upper));
                        if !has_level {
                            return false;
                        }
                    }
                    // Grep filter
                    if let Some(ref pattern) = grep_filter {
                        if !line.to_lowercase().contains(&pattern.to_lowercase()) {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            if filtered.is_empty() {
                println!("No log entries match the specified filters.");
                return 0;
            }

            match output_format {
                "json" => {
                    // Output as JSON array
                    let json = serde_json::json!(filtered);
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                _ => {
                    // Text output
                    for line in filtered {
                        println!("{}", line);
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Error reading logs: {}", e);
            1
        }
    }
}

/// Show log statistics
fn handle_logs_stats() -> i32 {
    match logging::get_log_stats() {
        Ok(stats) => {
            println!("Log Statistics:");
            println!("  Total files: {}", stats.total_files);
            println!(
                "  Total size: {}",
                logging::format_size(stats.total_size_bytes)
            );
            println!(
                "  Current file size: {}",
                logging::format_size(stats.current_file_size_bytes)
            );

            if !stats.entries_by_level.is_empty() {
                println!("\n  Entries by level:");
                for (level, count) in &stats.entries_by_level {
                    println!("    {}: {}", level, count);
                }
            }

            if let Some(ref oldest) = stats.oldest_entry {
                let truncated: String = oldest.chars().take(80).collect();
                println!("\n  Oldest entry: {}...", truncated);
            }
            if let Some(ref newest) = stats.newest_entry {
                let truncated: String = newest.chars().take(80).collect();
                println!("  Newest entry: {}...", truncated);
            }

            0
        }
        Err(e) => {
            eprintln!("Error getting log stats: {}", e);
            1
        }
    }
}

/// Default max age for log cleanup (30 days)
const DEFAULT_MAX_AGE_DAYS: u32 = 30;

/// Clean old log files
fn handle_logs_clean(all: bool, dry_run: bool) -> i32 {
    let log_dir = logging::default_log_directory();

    if dry_run {
        // Just show what would be removed
        if !log_dir.exists() {
            println!("No log directory found.");
            return 0;
        }

        let mut would_remove = 0;
        let mut would_remove_bytes: u64 = 0;
        let max_age_secs = DEFAULT_MAX_AGE_DAYS as u64 * 24 * 60 * 60;
        let now = std::time::SystemTime::now();

        if let Ok(entries) = std::fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let should_remove = if all {
                        true
                    } else if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                age.as_secs() > max_age_secs
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if should_remove {
                        if let Ok(metadata) = path.metadata() {
                            would_remove_bytes += metadata.len();
                        }
                        would_remove += 1;
                        println!("Would remove: {}", path.display());
                    }
                }
            }
        }

        if would_remove > 0 {
            println!(
                "\nWould remove {} files ({})",
                would_remove,
                logging::format_size(would_remove_bytes)
            );
        } else {
            println!("No files would be removed.");
        }
        return 0;
    }

    match logging::clean_logs(DEFAULT_MAX_AGE_DAYS, all) {
        Ok((removed, bytes)) => {
            if removed > 0 {
                println!(
                    "Removed {} log files ({})",
                    removed,
                    logging::format_size(bytes)
                );
            } else {
                println!("No log files to clean.");
            }
            0
        }
        Err(e) => {
            eprintln!("Error cleaning logs: {}", e);
            1
        }
    }
}

/// Show logging configuration
fn handle_logs_config() -> i32 {
    let log_dir = logging::default_log_directory();
    let log_file = logging::current_log_file();

    println!("Logging Configuration:");
    println!("  Directory: {}", log_dir.display());
    println!("  Current file: {}", log_file.display());
    println!("  Rotation: Daily");
    println!("  Format: JSON (file), Text/JSON (console)");
    println!("  Cleanup max age: {} days", DEFAULT_MAX_AGE_DAYS);
    println!();
    println!("Logs are written through the unified tracing system.");
    println!("Use --debug or --trace flags to increase verbosity.");

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_config() {
        // Should not panic
        let result = handle_logs_config();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_logs_stats() {
        // Should not panic even with no logs
        let _result = handle_logs_stats();
    }
}
