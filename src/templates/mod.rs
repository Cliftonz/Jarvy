//! Template system for Jarvy
//!
//! This module provides:
//! - Template schema definition
//! - Template loading from TOML files
//! - Built-in template catalog

pub mod builtin;
pub mod schema;

// Public API exports
#[allow(unused_imports)]
pub use builtin::{BuiltinTemplate, get_builtin_template, list_builtin_templates};
#[allow(unused_imports)]
pub use schema::{Template, TemplateMeta, TemplateTools};
