//! AI agent detection for skill installation. Re-exports the canonical
//! [`crate::agents::Agent`] enum as `SkillAgent` (review item 19) — the
//! prior independent enum carried the same six variants and the same
//! filesystem-path mapping as the merged canonical type. Cross-subsystem
//! drift is now a compile error (a new variant added in one place lights
//! up everywhere).

pub use crate::agents::Agent as SkillAgent;

/// Detect every installed agent. Returns in `Agent::ALL` order.
pub fn detect_agents() -> Vec<SkillAgent> {
    SkillAgent::ALL
        .iter()
        .copied()
        .filter(|a| a.is_installed())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[serial_test::serial(jarvy_home_env)]
    fn detect_agents_empty_when_no_dirs() {
        // SAFETY: JARVY_HOME points at an empty tempdir; no agent dirs.
        #[allow(unsafe_code)]
        unsafe {
            let tmp = tempdir().unwrap();
            std::env::set_var("JARVY_HOME", tmp.path());
            let agents = detect_agents();
            assert!(agents.is_empty(), "got {agents:?}");
            std::env::remove_var("JARVY_HOME");
        }
    }

    #[test]
    #[serial_test::serial(jarvy_home_env)]
    fn detect_agents_finds_present_dirs() {
        // SAFETY: scoped JARVY_HOME for this test only.
        #[allow(unsafe_code)]
        unsafe {
            let tmp = tempdir().unwrap();
            std::fs::create_dir(tmp.path().join(".claude")).unwrap();
            std::fs::create_dir(tmp.path().join(".cursor")).unwrap();
            std::env::set_var("JARVY_HOME", tmp.path());
            let agents = detect_agents();
            assert!(agents.contains(&SkillAgent::ClaudeCode));
            assert!(agents.contains(&SkillAgent::Cursor));
            assert!(!agents.contains(&SkillAgent::Codex));
            std::env::remove_var("JARVY_HOME");
        }
    }
}
