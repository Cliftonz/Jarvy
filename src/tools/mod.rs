pub mod brew;
pub mod common;
pub mod docker;
pub mod git;
pub mod registry;
pub mod vscode;

#[allow(unused_imports)]
pub use common::{
    InstallError, Os, PackageManager, PkgOps, cmd_satisfies, current_os, has, require, require_any,
    run,
};

#[allow(unused_imports)]
pub use registry::{ToolAdder, add, get_tool, register_tool};

/// Register all built-in tools to the registry. Call this early in the program init if you
/// want to add("git", ...)/add("docker", ...) to work without manual registration.
pub fn register_all() {
    // Each tool exposes a small add_handler(version) function we can register.
    // Ignore duplicate returns; the last one wins.
    let _ = register_tool("git", crate::tools::git::git::add_handler);
    let _ = register_tool("brew", crate::tools::brew::brew::add_handler);
    let _ = register_tool("vscode", crate::tools::vscode::vscode::add_handler);
    let _ = register_tool("docker", crate::tools::docker::docker::add_handler);
}
