//! docker - containerization platform
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(DOCKER, {
    command: "docker",
    macos: { cask: "docker" },
    linux: { apt: "docker.io", dnf: "docker", pacman: "docker", apk: "docker" },
    windows: { winget: "Docker.DockerDesktop" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_docker_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
