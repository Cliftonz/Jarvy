//! AI agent detection for skill installation.
//!
//! Same agent set as `crate::ai_hooks::AgentTarget` (claude-code,
//! cursor, codex, windsurf, cline, continue) but with skills-specific
//! filesystem paths. Kept independent so changes to one subsystem
//! don't accidentally break the other.

use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillAgent {
    ClaudeCode,
    Cursor,
    Codex,
    Windsurf,
    Cline,
    Continue,
}

impl SkillAgent {
    pub const ALL: &'static [SkillAgent] = &[
        SkillAgent::ClaudeCode,
        SkillAgent::Cursor,
        SkillAgent::Codex,
        SkillAgent::Windsurf,
        SkillAgent::Cline,
        SkillAgent::Continue,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            SkillAgent::ClaudeCode => "claude-code",
            SkillAgent::Cursor => "cursor",
            SkillAgent::Codex => "codex",
            SkillAgent::Windsurf => "windsurf",
            SkillAgent::Cline => "cline",
            SkillAgent::Continue => "continue",
        }
    }

    pub fn from_slug(slug: &str) -> Option<SkillAgent> {
        Self::ALL
            .iter()
            .copied()
            .find(|a| a.slug().eq_ignore_ascii_case(slug))
    }

    /// Agent's config directory under `$HOME` (or `JARVY_HOME` for
    /// tests). Returns `None` if home lookup fails.
    pub fn config_dir(self) -> Option<PathBuf> {
        let home = home_dir()?;
        Some(match self {
            SkillAgent::ClaudeCode => home.join(".claude"),
            SkillAgent::Cursor => home.join(".cursor"),
            SkillAgent::Codex => home.join(".codex"),
            SkillAgent::Windsurf => home.join(".windsurf"),
            SkillAgent::Cline => home.join(".cline"),
            SkillAgent::Continue => home.join(".continue"),
        })
    }

    /// Where skills land for this agent.
    pub fn skills_dir(self) -> Option<PathBuf> {
        self.config_dir().map(|p| p.join("skills"))
    }

    /// `true` when the agent's config directory exists on disk —
    /// proxy for "agent is installed."
    pub fn is_installed(self) -> bool {
        self.config_dir().map(|p| p.exists()).unwrap_or(false)
    }
}

/// Detect every installed agent. Returns in `ALL` order.
pub fn detect_agents() -> Vec<SkillAgent> {
    SkillAgent::ALL
        .iter()
        .copied()
        .filter(|a| a.is_installed())
        .collect()
}

/// Honors `JARVY_HOME` for tests; otherwise standard `HOME` /
/// `USERPROFILE` lookup.
fn home_dir() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("JARVY_HOME") {
        return Some(PathBuf::from(v));
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn slug_round_trips() {
        for a in SkillAgent::ALL {
            assert_eq!(SkillAgent::from_slug(a.slug()), Some(*a));
        }
    }

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
