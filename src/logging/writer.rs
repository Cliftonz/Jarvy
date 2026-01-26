//! Rotating file writer for log output
//!
//! Handles writing log entries to files with rotation support.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::LogError;
use super::config::LoggingConfig;
use super::rotator::LogRotator;

/// Thread-safe rotating file writer
pub struct RotatingFileWriter {
    /// Path to the current log file
    path: PathBuf,
    /// Inner writer wrapped in mutex for thread safety
    inner: Arc<Mutex<Option<BufWriter<File>>>>,
    /// Current file size in bytes
    current_size: Arc<Mutex<u64>>,
    /// Configuration
    config: LoggingConfig,
    /// Log rotator
    rotator: LogRotator,
}

impl RotatingFileWriter {
    /// Create a new rotating file writer
    pub fn new(config: LoggingConfig) -> Result<Self, LogError> {
        let log_dir = config.directory.clone();
        std::fs::create_dir_all(&log_dir)?;

        let path = log_dir.join("jarvy.log");
        let rotator = LogRotator::new(config.clone());

        let mut writer = Self {
            path,
            inner: Arc::new(Mutex::new(None)),
            current_size: Arc::new(Mutex::new(0)),
            config,
            rotator,
        };

        writer.open_file()?;
        Ok(writer)
    }

    /// Open or reopen the log file
    fn open_file(&mut self) -> Result<(), LogError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| LogError::FileOpenFailed(e.to_string()))?;

        let size = file.metadata().map(|m| m.len()).unwrap_or(0);

        let buffered = BufWriter::new(file);

        {
            let mut inner = self.inner.lock().unwrap();
            *inner = Some(buffered);
        }
        {
            let mut current_size = self.current_size.lock().unwrap();
            *current_size = size;
        }

        Ok(())
    }

    /// Write a log line to the file
    pub fn write_line(&self, line: &str) -> Result<(), LogError> {
        let line_bytes = line.as_bytes();
        let line_len = line_bytes.len() as u64 + 1; // +1 for newline

        // Check if rotation is needed before writing
        {
            let current_size = self.current_size.lock().unwrap();
            if *current_size + line_len > self.config.max_file_size {
                drop(current_size);
                self.rotate()?;
            }
        }

        // Write the line
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(ref mut writer) = *inner {
                writer
                    .write_all(line_bytes)
                    .map_err(|e| LogError::WriteFailed(e.to_string()))?;
                writer
                    .write_all(b"\n")
                    .map_err(|e| LogError::WriteFailed(e.to_string()))?;
                writer
                    .flush()
                    .map_err(|e| LogError::WriteFailed(e.to_string()))?;
            }
        }

        // Update size
        {
            let mut current_size = self.current_size.lock().unwrap();
            *current_size += line_len;
        }

        Ok(())
    }

    /// Force rotation of the log file
    pub fn rotate(&self) -> Result<(), LogError> {
        // Close the current file
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(ref mut writer) = *inner {
                let _ = writer.flush();
            }
            *inner = None;
        }

        // Perform rotation
        self.rotator.rotate()?;

        // Reopen file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| LogError::FileOpenFailed(e.to_string()))?;

        let buffered = BufWriter::new(file);

        {
            let mut inner = self.inner.lock().unwrap();
            *inner = Some(buffered);
        }
        {
            let mut current_size = self.current_size.lock().unwrap();
            *current_size = 0;
        }

        Ok(())
    }

    /// Flush any buffered data
    pub fn flush(&self) -> Result<(), LogError> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(ref mut writer) = *inner {
            writer
                .flush()
                .map_err(|e| LogError::WriteFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Get the current log file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the current file size
    pub fn current_size(&self) -> u64 {
        *self.current_size.lock().unwrap()
    }
}

impl Clone for RotatingFileWriter {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            inner: Arc::clone(&self.inner),
            current_size: Arc::clone(&self.current_size),
            config: self.config.clone(),
            rotator: self.rotator.clone(),
        }
    }
}

impl Drop for RotatingFileWriter {
    fn drop(&mut self) {
        let _ = self.flush();
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
            max_file_size: 1024, // 1KB for testing
            max_files: 3,
            ..Default::default()
        }
    }

    #[test]
    fn test_create_writer() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let writer = RotatingFileWriter::new(config).unwrap();

        assert!(writer.path().exists() || writer.path().parent().unwrap().exists());
    }

    #[test]
    fn test_write_line() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let writer = RotatingFileWriter::new(config).unwrap();

        writer.write_line("Test log entry").unwrap();
        writer.flush().unwrap();

        let content = std::fs::read_to_string(writer.path()).unwrap();
        assert!(content.contains("Test log entry"));
    }

    #[test]
    fn test_size_tracking() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let writer = RotatingFileWriter::new(config).unwrap();

        let initial_size = writer.current_size();
        writer.write_line("Test log entry").unwrap();

        assert!(writer.current_size() > initial_size);
    }

    #[test]
    fn test_clone_shares_state() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let writer1 = RotatingFileWriter::new(config).unwrap();
        let writer2 = writer1.clone();

        writer1.write_line("From writer 1").unwrap();

        // Both writers should see the same size
        assert_eq!(writer1.current_size(), writer2.current_size());
    }
}
