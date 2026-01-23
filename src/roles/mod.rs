//! Role-Based Configurations Module (PRD-033)
//!
//! This module provides:
//! - Role definitions with named tool sets (`[roles.name]`)
//! - Role assignment via `role = "name"` or `roles = ["a", "b"]`
//! - Role inheritance with `extends = "parent"` or `extends = ["a", "b"]`
//! - CLI commands: `jarvy roles list/show/diff`
//! - Role override via `--role` flag

pub mod commands;
pub mod definition;
pub mod resolver;

pub use commands::{RolesAction, handle_roles_command};
#[allow(unused_imports)]
pub use definition::{RoleAssignment, RoleDefinition, RolesConfig};
#[allow(unused_imports)]
pub use resolver::{ResolvedRole, RoleResolver, RoleResolverError};

/// Maximum inheritance depth to prevent infinite recursion
pub const MAX_INHERITANCE_DEPTH: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_depth_constant() {
        assert_eq!(MAX_INHERITANCE_DEPTH, 5);
    }
}
