//! Error types for the observability module

#![allow(dead_code)] // Public API for observability error handling

use thiserror::Error;

/// Errors that can occur during observability operations
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// IO error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// ZIP archive error
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Tracing subscriber initialization error
    #[error("tracing initialization error: {0}")]
    Tracing(#[from] tracing::subscriber::SetGlobalDefaultError),

    /// Logging configuration error
    #[error("logging configuration error: {0}")]
    LogConfig(String),
}
