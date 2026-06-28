//! AI agent skill installation (PRD-049 v1, riding on PRD-054 library
//! registry).
//!
//! Skills are markdown files (`SKILL.md` + optional companions) that
//! live under each AI coding agent's config directory
//! (`~/.claude/skills/`, `~/.cursor/skills/`, etc.). Jarvy installs them
//! by fetching the `skill_md_url` published in a library manifest,
//! sha256-verifying against the manifest entry, and writing to every
//! detected agent's skill directory.
//!
//! # v1 scope (this module ships)
//!
//! - `[skills]` config block with `library_sources` + `install` list
//! - `jarvy skills {install, list, status}` subcommand
//! - Auto-install during `jarvy setup` (gated on
//!   `[skills] auto_install = true`)
//! - Agent detection (claude-code, cursor, codex, windsurf, cline,
//!   continue — same set as ai_hooks)
//! - sha256 verification of fetched `SKILL.md`
//!
//! # Out of scope for v1 (PRD-049 follow-up)
//!
//! - skills.sh API integration (search / popular / info commands)
//! - Version-pin upgrades (`jarvy skills update`)
//! - Companion file fetching (only `SKILL.md` lands today)
//! - Project-scope skills (only `~/.agent/skills/` user scope)

pub mod agents;
pub mod config;
pub mod installer;

pub use agents::{SkillAgent, detect_agents};
pub use config::{SkillEntry, SkillsConfig};
#[allow(unused_imports)] // Public lib API; bin only uses install_skill + SkillStatus directly
pub use installer::{InstallResult, SkillError};
pub use installer::{SkillStatus, install_skill};
