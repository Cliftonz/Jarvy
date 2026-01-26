//! Log rotation and cleanup
//!
//! Handles rotating log files by size and cleaning up old logs.

use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use super::LogError;
use super::config::LoggingConfig;

/// Log file rotator
#[derive(Debug, Clone)]
pub struct LogRotator {
    config: LoggingConfig,
}

impl LogRotator {
    /// Create a new log rotator with the given configuration
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// Rotate the current log file
    ///
    /// Renames jarvy.log -> jarvy.log.1.gz, jarvy.log.1.gz -> jarvy.log.2.gz, etc.
    pub fn rotate(&self) -> Result<(), LogError> {
        let log_dir = &self.config.directory;
        let current_log = log_dir.join("jarvy.log");

        if !current_log.exists() {
            return Ok(());
        }

        // Shift existing rotated files
        self.shift_rotated_files()?;

        // Compress and rename current log to .1.gz
        let rotated_path = log_dir.join("jarvy.log.1.gz");
        self.compress_file(&current_log, &rotated_path)?;

        // Remove the original uncompressed file
        fs::remove_file(&current_log).map_err(|e| {
            LogError::RotationFailed(format!("failed to remove original log: {}", e))
        })?;

        // Clean up old files based on max_files and max_total_size
        self.enforce_limits()?;

        // Clean up files older than max_age_days
        self.cleanup_old_logs()?;

        Ok(())
    }

    /// Shift existing rotated files (1 -> 2, 2 -> 3, etc.)
    fn shift_rotated_files(&self) -> Result<(), LogError> {
        let log_dir = &self.config.directory;

        // Start from the highest number and work down
        for i in (1..self.config.max_files).rev() {
            let from = log_dir.join(format!("jarvy.log.{}.gz", i));
            let to = log_dir.join(format!("jarvy.log.{}.gz", i + 1));

            if from.exists() {
                fs::rename(&from, &to).map_err(|e| {
                    LogError::RotationFailed(format!(
                        "failed to rename {} to {}: {}",
                        from.display(),
                        to.display(),
                        e
                    ))
                })?;
            }
        }

        Ok(())
    }

    /// Compress a file using gzip
    fn compress_file(&self, source: &PathBuf, dest: &PathBuf) -> Result<(), LogError> {
        let input_file = File::open(source)
            .map_err(|e| LogError::CompressionFailed(format!("failed to open source: {}", e)))?;
        let mut reader = BufReader::new(input_file);

        let output_file = File::create(dest)
            .map_err(|e| LogError::CompressionFailed(format!("failed to create dest: {}", e)))?;
        let mut encoder = GzEncoder::new(output_file, Compression::default());

        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|e| {
                LogError::CompressionFailed(format!("failed to read source: {}", e))
            })?;
            if bytes_read == 0 {
                break;
            }
            encoder.write_all(&buffer[..bytes_read]).map_err(|e| {
                LogError::CompressionFailed(format!("failed to write compressed data: {}", e))
            })?;
        }

        encoder.finish().map_err(|e| {
            LogError::CompressionFailed(format!("failed to finish compression: {}", e))
        })?;

        Ok(())
    }

    /// Enforce max_files and max_total_size limits
    fn enforce_limits(&self) -> Result<(), LogError> {
        let log_dir = &self.config.directory;

        // Delete files beyond max_files (1-indexed, so max_files=2 keeps 1 and 2, deletes 3+)
        for i in (self.config.max_files + 1)..100 {
            let path = log_dir.join(format!("jarvy.log.{}.gz", i));
            if path.exists() {
                fs::remove_file(&path).ok();
            } else {
                break;
            }
        }

        // Enforce max_total_size
        let mut total_size = self.calculate_total_size()?;
        let mut i = self.config.max_files;

        while total_size > self.config.max_total_size && i > 0 {
            let path = log_dir.join(format!("jarvy.log.{}.gz", i));
            if path.exists() {
                if let Ok(metadata) = path.metadata() {
                    total_size = total_size.saturating_sub(metadata.len());
                    fs::remove_file(&path).ok();
                }
            }
            i -= 1;
        }

        Ok(())
    }

    /// Calculate total size of all log files
    fn calculate_total_size(&self) -> Result<u64, LogError> {
        let log_dir = &self.config.directory;
        let mut total: u64 = 0;

        if !log_dir.exists() {
            return Ok(0);
        }

        for entry in (fs::read_dir(log_dir)
            .map_err(|e| LogError::FileOpenFailed(format!("failed to read log dir: {}", e)))?)
        .flatten()
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with("jarvy.log"))
                    .unwrap_or(false)
            {
                if let Ok(metadata) = path.metadata() {
                    total += metadata.len();
                }
            }
        }

        Ok(total)
    }

    /// Clean up log files older than max_age_days
    pub fn cleanup_old_logs(&self) -> Result<(), LogError> {
        let log_dir = &self.config.directory;
        let max_age_secs = self.config.max_age_days as u64 * 24 * 60 * 60;
        let now = SystemTime::now();

        if !log_dir.exists() {
            return Ok(());
        }

        for entry in (fs::read_dir(log_dir)
            .map_err(|e| LogError::FileOpenFailed(format!("failed to read log dir: {}", e)))?)
        .flatten()
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .map(|n| {
                        let name = n.to_string_lossy();
                        name.starts_with("jarvy.log.") && name.ends_with(".gz")
                    })
                    .unwrap_or(false)
            {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age.as_secs() > max_age_secs {
                                fs::remove_file(&path).ok();
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get a list of all log files with their sizes
    pub fn list_log_files(&self) -> Result<Vec<(PathBuf, u64)>, LogError> {
        let log_dir = &self.config.directory;
        let mut files = Vec::new();

        if !log_dir.exists() {
            return Ok(files);
        }

        for entry in (fs::read_dir(log_dir)
            .map_err(|e| LogError::FileOpenFailed(format!("failed to read log dir: {}", e)))?)
        .flatten()
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with("jarvy.log"))
                    .unwrap_or(false)
            {
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                files.push((path, size));
            }
        }

        // Sort by name (current log first, then by number)
        files.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(dir: &TempDir) -> LoggingConfig {
        LoggingConfig {
            enabled: true,
            directory: dir.path().to_path_buf(),
            max_file_size: 1024,
            max_files: 3,
            max_total_size: 10 * 1024,
            max_age_days: 30,
            ..Default::default()
        }
    }

    #[test]
    fn test_rotate_nonexistent() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let rotator = LogRotator::new(config);

        // Should not error on nonexistent file
        rotator.rotate().unwrap();
    }

    #[test]
    fn test_rotate_creates_compressed() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let rotator = LogRotator::new(config.clone());

        // Create a log file
        let log_path = config.directory.join("jarvy.log");
        fs::write(&log_path, "Test log content\n").unwrap();

        rotator.rotate().unwrap();

        // Original should be gone
        assert!(!log_path.exists());

        // Compressed version should exist
        let compressed = config.directory.join("jarvy.log.1.gz");
        assert!(compressed.exists());
    }

    #[test]
    fn test_shift_rotated_files() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let rotator = LogRotator::new(config.clone());

        // Create initial rotated files
        fs::write(config.directory.join("jarvy.log.1.gz"), "old1").unwrap();
        fs::write(config.directory.join("jarvy.log.2.gz"), "old2").unwrap();

        rotator.shift_rotated_files().unwrap();

        // Files should have shifted
        assert!(!config.directory.join("jarvy.log.1.gz").exists());
        assert!(config.directory.join("jarvy.log.2.gz").exists());
        assert!(config.directory.join("jarvy.log.3.gz").exists());
    }

    #[test]
    fn test_enforce_max_files() {
        let dir = TempDir::new().unwrap();
        let mut config = test_config(&dir);
        config.max_files = 2;
        let rotator = LogRotator::new(config.clone());

        // Create more files than max
        fs::write(config.directory.join("jarvy.log.1.gz"), "1").unwrap();
        fs::write(config.directory.join("jarvy.log.2.gz"), "2").unwrap();
        fs::write(config.directory.join("jarvy.log.3.gz"), "3").unwrap();

        rotator.enforce_limits().unwrap();

        // Only max_files should remain
        assert!(config.directory.join("jarvy.log.1.gz").exists());
        assert!(config.directory.join("jarvy.log.2.gz").exists());
        assert!(!config.directory.join("jarvy.log.3.gz").exists());
    }

    #[test]
    fn test_calculate_total_size() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let rotator = LogRotator::new(config.clone());

        // Create some files
        fs::write(config.directory.join("jarvy.log"), "current").unwrap();
        fs::write(config.directory.join("jarvy.log.1.gz"), "old").unwrap();

        let size = rotator.calculate_total_size().unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_list_log_files() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let rotator = LogRotator::new(config.clone());

        fs::write(config.directory.join("jarvy.log"), "current").unwrap();
        fs::write(config.directory.join("jarvy.log.1.gz"), "1").unwrap();
        fs::write(config.directory.join("other.txt"), "other").unwrap();

        let files = rotator.list_log_files().unwrap();

        // Should only include jarvy.log files
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|(p, _)| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("jarvy.log")
        }));
    }
}
