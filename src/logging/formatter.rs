//! Log formatters for different output formats
//!
//! Supports JSON and human-readable text formats.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt::Write;

use super::config::LogFormat;
use super::sanitizer::Sanitizer;

/// A formatted log entry
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub fields: std::collections::HashMap<String, serde_json::Value>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: &str, target: &str, message: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            level: level.to_uppercase(),
            target: target.to_string(),
            message: message.to_string(),
            span: None,
            fields: std::collections::HashMap::new(),
        }
    }

    /// Add a span name to the entry
    pub fn with_span(mut self, span: &str) -> Self {
        self.span = Some(span.to_string());
        self
    }

    /// Add a field to the entry
    pub fn with_field(mut self, key: &str, value: serde_json::Value) -> Self {
        self.fields.insert(key.to_string(), value);
        self
    }
}

/// Log formatter that produces formatted log output
pub struct LogFormatter {
    format: LogFormat,
    sanitizer: Sanitizer,
}

impl LogFormatter {
    /// Create a new formatter with the specified format
    pub fn new(format: LogFormat) -> Self {
        Self {
            format,
            sanitizer: Sanitizer::new(),
        }
    }

    /// Format a log entry to a string
    pub fn format(&self, entry: &LogEntry) -> String {
        let formatted = match self.format {
            LogFormat::Json => self.format_json(entry),
            LogFormat::Text => self.format_text(entry),
        };
        self.sanitizer.sanitize(&formatted).to_string()
    }

    /// Format as JSON
    fn format_json(&self, entry: &LogEntry) -> String {
        serde_json::to_string(entry).unwrap_or_else(|_| {
            format!(
                r#"{{"timestamp":"{}","level":"{}","message":"serialization error"}}"#,
                entry.timestamp.to_rfc3339(),
                entry.level
            )
        })
    }

    /// Format as human-readable text
    fn format_text(&self, entry: &LogEntry) -> String {
        let mut output = String::with_capacity(256);

        // Timestamp
        let _ = write!(
            output,
            "{} ",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f")
        );

        // Level with padding
        let _ = write!(output, "{:5} ", entry.level);

        // Target
        let _ = write!(output, "[{}] ", entry.target);

        // Span if present
        if let Some(ref span) = entry.span {
            let _ = write!(output, "({}) ", span);
        }

        // Message
        output.push_str(&entry.message);

        // Fields if present
        if !entry.fields.is_empty() {
            output.push_str(" {");
            let mut first = true;
            for (key, value) in &entry.fields {
                if !first {
                    output.push_str(", ");
                }
                first = false;
                let _ = write!(output, "{}={}", key, value);
            }
            output.push('}');
        }

        output
    }
}

impl Default for LogFormatter {
    fn default() -> Self {
        Self::new(LogFormat::Json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new("info", "jarvy::setup", "Installing tools");
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.target, "jarvy::setup");
        assert_eq!(entry.message, "Installing tools");
        assert!(entry.span.is_none());
        assert!(entry.fields.is_empty());
    }

    #[test]
    fn test_log_entry_with_span() {
        let entry = LogEntry::new("debug", "jarvy", "test").with_span("my_span");
        assert_eq!(entry.span, Some("my_span".to_string()));
    }

    #[test]
    fn test_log_entry_with_field() {
        let entry =
            LogEntry::new("info", "jarvy", "test").with_field("count", serde_json::json!(42));
        assert_eq!(entry.fields.get("count"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_json_format() {
        let formatter = LogFormatter::new(LogFormat::Json);
        let entry = LogEntry::new("info", "test", "Hello world");
        let output = formatter.format(&entry);

        assert!(output.contains("\"level\":\"INFO\""));
        assert!(output.contains("\"message\":\"Hello world\""));
        assert!(output.contains("\"target\":\"test\""));
    }

    #[test]
    fn test_text_format() {
        let formatter = LogFormatter::new(LogFormat::Text);
        let entry = LogEntry::new("warn", "jarvy::tools", "Tool not found");
        let output = formatter.format(&entry);

        assert!(output.contains("WARN"));
        assert!(output.contains("[jarvy::tools]"));
        assert!(output.contains("Tool not found"));
    }

    #[test]
    fn test_text_format_with_span() {
        let formatter = LogFormatter::new(LogFormat::Text);
        let entry = LogEntry::new("debug", "test", "message").with_span("install");
        let output = formatter.format(&entry);

        assert!(output.contains("(install)"));
    }

    #[test]
    fn test_text_format_with_fields() {
        let formatter = LogFormatter::new(LogFormat::Text);
        let entry =
            LogEntry::new("info", "test", "message").with_field("tool", serde_json::json!("git"));
        let output = formatter.format(&entry);

        assert!(output.contains("tool="));
        assert!(output.contains("\"git\""));
    }
}
