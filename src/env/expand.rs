//! Variable expansion for environment values
//!
//! Supports:
//! - $HOME, $PWD, $USER expansion
//! - ${VAR} and $VAR syntax
//! - JARVY_{TOOL}_PATH expansion for tool paths
//! - Existing environment variable expansion

use std::collections::HashMap;
use std::path::PathBuf;

/// Context for variable expansion
#[derive(Debug, Clone)]
pub struct EnvContext {
    /// Home directory
    pub home_dir: PathBuf,
    /// Current working directory
    pub current_dir: PathBuf,
    /// Current username
    pub username: String,
    /// Tool paths keyed by tool name (e.g., "node" -> "/usr/local/bin/node")
    pub tool_paths: HashMap<String, PathBuf>,
    /// Additional custom variables
    pub custom_vars: HashMap<String, String>,
}

impl Default for EnvContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvContext {
    /// Create a new EnvContext with system values
    pub fn new() -> Self {
        Self {
            home_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")),
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            username: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string()),
            tool_paths: HashMap::new(),
            custom_vars: HashMap::new(),
        }
    }

    /// Add a tool path to the context
    pub fn with_tool_path(mut self, tool: &str, path: PathBuf) -> Self {
        self.tool_paths.insert(tool.to_string(), path);
        self
    }

    /// Add a custom variable
    pub fn with_var(mut self, key: &str, value: &str) -> Self {
        self.custom_vars.insert(key.to_string(), value.to_string());
        self
    }
}

/// Expand variables in a string value
///
/// Supports:
/// - `$HOME` -> user's home directory
/// - `$PWD` -> current working directory
/// - `$USER` -> current username
/// - `$VAR` -> value of environment variable VAR
/// - `${VAR}` -> same as $VAR, allows concatenation like `${HOME}/bin`
/// - `$JARVY_TOOL_PATH` -> path to an installed tool (from context)
///
/// # Arguments
/// * `value` - The string containing variables to expand
/// * `ctx` - The expansion context containing system values
///
/// # Returns
/// The expanded string with all variables replaced
pub fn expand_value(value: &str, ctx: &EnvContext) -> String {
    let mut result = value.to_string();

    // First, expand ${VAR} syntax (braced variables)
    let braced_re = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
    result = braced_re
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            resolve_variable(var_name, ctx)
        })
        .to_string();

    // Then, expand $VAR syntax (unbraced variables)
    // Be careful not to match things like $123 or inside already expanded values
    let unbraced_re = regex::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    result = unbraced_re
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            resolve_variable(var_name, ctx)
        })
        .to_string();

    result
}

/// Resolve a single variable name to its value
fn resolve_variable(name: &str, ctx: &EnvContext) -> String {
    // Check built-in variables first
    match name {
        "HOME" => return ctx.home_dir.to_string_lossy().to_string(),
        "PWD" => return ctx.current_dir.to_string_lossy().to_string(),
        "USER" => return ctx.username.clone(),
        _ => {}
    }

    // Check for JARVY_{TOOL}_PATH pattern
    if name.starts_with("JARVY_") && name.ends_with("_PATH") {
        let tool_name = name
            .strip_prefix("JARVY_")
            .unwrap()
            .strip_suffix("_PATH")
            .unwrap()
            .to_lowercase();
        if let Some(path) = ctx.tool_paths.get(&tool_name) {
            return path.to_string_lossy().to_string();
        }
    }

    // Check custom variables
    if let Some(value) = ctx.custom_vars.get(name) {
        return value.clone();
    }

    // Fall back to environment variable
    std::env::var(name).unwrap_or_else(|_| format!("${}", name))
}

/// Expand a path, handling ~ for home directory
pub fn expand_path(path: &str, ctx: &EnvContext) -> PathBuf {
    let expanded = if path.starts_with('~') {
        path.replacen('~', &ctx.home_dir.to_string_lossy(), 1)
    } else {
        path.to_string()
    };

    PathBuf::from(expand_value(&expanded, ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home() {
        let ctx = EnvContext::new();
        let result = expand_value("$HOME/bin", &ctx);
        assert!(result.contains("/bin"));
        assert!(!result.contains("$HOME"));
    }

    #[test]
    fn test_expand_braced() {
        let ctx = EnvContext::new();
        let result = expand_value("${HOME}/projects/${USER}", &ctx);
        assert!(!result.contains("${"));
        assert!(result.contains("/projects/"));
    }

    #[test]
    fn test_expand_pwd() {
        let ctx = EnvContext::new();
        let result = expand_value("$PWD/.env", &ctx);
        assert!(result.ends_with("/.env"));
        assert!(!result.contains("$PWD"));
    }

    #[test]
    fn test_expand_custom_var() {
        let ctx = EnvContext::new().with_var("MY_VAR", "my_value");
        let result = expand_value("prefix_${MY_VAR}_suffix", &ctx);
        assert_eq!(result, "prefix_my_value_suffix");
    }

    #[test]
    fn test_expand_tool_path() {
        let ctx = EnvContext::new().with_tool_path("node", PathBuf::from("/usr/local/bin/node"));
        let result = expand_value("$JARVY_NODE_PATH", &ctx);
        assert_eq!(result, "/usr/local/bin/node");
    }

    #[test]
    fn test_expand_unknown_var() {
        let ctx = EnvContext::new();
        let result = expand_value("$NONEXISTENT_VAR_12345", &ctx);
        // Should keep the original if not found
        assert_eq!(result, "$NONEXISTENT_VAR_12345");
    }

    #[test]
    fn test_expand_path_tilde() {
        let ctx = EnvContext::new();
        let result = expand_path("~/.config/app", &ctx);
        assert!(!result.to_string_lossy().contains('~'));
        assert!(result.to_string_lossy().contains(".config/app"));
    }

    #[test]
    fn test_expand_multiple_vars() {
        let ctx = EnvContext::new()
            .with_var("PROJECT", "myapp")
            .with_var("VERSION", "1.0");
        let result = expand_value("${PROJECT}-${VERSION}", &ctx);
        assert_eq!(result, "myapp-1.0");
    }

    #[test]
    fn test_env_context_default() {
        let ctx = EnvContext::default();
        assert!(!ctx.home_dir.as_os_str().is_empty());
        assert!(!ctx.username.is_empty());
    }
}
