//! Error type for MCP server registration.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpRegisterError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse existing settings at {path}: {source}")]
    ParseExisting {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to parse existing TOML at {path}: {source}")]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to serialize: {0}")]
    Serialize(#[from] serde_json::Error),

    // Reserved for future per-agent platform refusals (e.g. Cline VS
    // Code variant not detected). Currently unused — every registrar
    // either supports the host or gracefully degrades.
    #[allow(dead_code)]
    #[error("agent '{0}' is not supported on this platform")]
    UnsupportedPlatform(&'static str),

    #[error("invalid server entry '{name}': {reason}")]
    InvalidEntry { name: String, reason: String },

    #[error(
        "settings file at {path} is a symlink — refusing to write through it. \
         Remove the symlink or set `JARVY_HOME` to a clean location."
    )]
    SettingsPathIsSymlink { path: PathBuf },
}

impl McpRegisterError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Stable telemetry tag identifying which variant fired.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::ParseExisting { .. } => "parse_existing",
            Self::ParseToml { .. } => "parse_toml",
            Self::Serialize(_) => "serialize",
            Self::UnsupportedPlatform(_) => "unsupported_platform",
            Self::InvalidEntry { .. } => "invalid_entry",
            Self::SettingsPathIsSymlink { .. } => "settings_path_is_symlink",
        }
    }
}

// Bridge from the AI hooks io helpers (atomic write, symlink refusal,
// `home_or_err`). They return `AiHookError`; map each variant onto the
// matching `McpRegisterError` so call sites can `?` cleanly.
impl From<crate::ai_hooks::AiHookError> for McpRegisterError {
    fn from(value: crate::ai_hooks::AiHookError) -> Self {
        use crate::ai_hooks::AiHookError as A;
        match value {
            A::Io { path, source } => Self::Io { path, source },
            A::ParseExisting { path, source } => Self::ParseExisting { path, source },
            A::Serialize(e) => Self::Serialize(e),
            A::InvalidEntry { name, reason } => Self::InvalidEntry { name, reason },
            A::SettingsPathIsSymlink { path } => Self::SettingsPathIsSymlink { path },
            // The remaining variants (UnknownLibraryHook, UnsupportedEvent,
            // UnsupportedPlatform) are unreachable from io helpers — map
            // defensively to InvalidEntry so the conversion stays total.
            other => Self::InvalidEntry {
                name: other.kind().to_string(),
                reason: other.to_string(),
            },
        }
    }
}
