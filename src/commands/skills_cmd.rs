//! `jarvy skills <action>` command handler (PRD-049 v1).

use crate::cli::SkillsAction;
use crate::config::Config;
use crate::skills::{self, SkillAgent, SkillEntry, SkillStatus, SkillsConfig};

pub fn run_skills(action: &SkillsAction, file: &str) -> i32 {
    let config = Config::new(file);
    let skills_cfg = config.skills.clone().unwrap_or_default();

    match action {
        SkillsAction::Install { name } => install_action(&skills_cfg, name.as_deref()),
        SkillsAction::List {} => list_action(&skills_cfg),
        SkillsAction::Status {} => status_action(&skills_cfg),
        SkillsAction::Agents {} => agents_action(),
    }
}

fn install_action(cfg: &SkillsConfig, only_name: Option<&str>) -> i32 {
    if cfg.install.is_empty() && only_name.is_none() {
        println!(
            "No skills configured. Add entries to `[skills.install]` in jarvy.toml \
             or pass `--name <skill>`."
        );
        return 0;
    }

    prepare_library_sources(cfg);

    let agents = resolve_target_agents(cfg);
    if agents.is_empty() {
        eprintln!(
            "No AI agents detected. Install Claude Code / Cursor / Codex / etc. first, \
             or check `jarvy skills agents`."
        );
        return crate::error_codes::CONFIG_ERROR;
    }

    let entries: Vec<(&String, &SkillEntry)> = match only_name {
        Some(want) => cfg
            .install
            .iter()
            .filter(|(name, _)| name.as_str() == want)
            .collect(),
        None => cfg.install.iter().collect(),
    };
    if entries.is_empty() {
        eprintln!(
            "Skill `{}` not found in `[skills.install]`.",
            only_name.unwrap_or("?"),
        );
        return crate::error_codes::CONFIG_ERROR;
    }

    let mut had_failure = false;
    for (name, entry) in entries {
        match skills::install_skill(name, entry, &agents) {
            Ok(result) => {
                println!(
                    "  Installed {} {} → {} agent(s)",
                    name,
                    result.version,
                    result.agents.len()
                );
                for skipped in &result.skipped_agents {
                    println!("    skipped {}: {}", skipped.0.slug(), skipped.1);
                }
            }
            Err(e) => {
                eprintln!("  Failed: {name}: {e}");
                had_failure = true;
            }
        }
    }
    if had_failure {
        crate::error_codes::CONFIG_ERROR
    } else {
        0
    }
}

fn list_action(cfg: &SkillsConfig) -> i32 {
    if cfg.install.is_empty() {
        println!("No skills configured in `[skills.install]`.");
        return 0;
    }
    println!("Configured skills ({}):", cfg.install.len());
    let agents = resolve_target_agents(cfg);
    for (name, entry) in &cfg.install {
        println!();
        println!("  {} = {}", name, entry.version());
        for agent in &agents {
            let status = skills::installer::skill_status(name, entry.version(), *agent);
            let label = match status {
                SkillStatus::Installed { version } => format!("installed ({version})"),
                SkillStatus::Missing => "missing".to_string(),
                SkillStatus::Drift {
                    installed,
                    requested,
                } => format!("drift: installed={installed} requested={requested}"),
            };
            println!("    {} → {}", agent.slug(), label);
        }
    }
    0
}

fn status_action(cfg: &SkillsConfig) -> i32 {
    let agents = resolve_target_agents(cfg);
    let mut drift_count = 0;
    let mut missing_count = 0;
    let mut installed_count = 0;
    for (name, entry) in &cfg.install {
        for agent in &agents {
            match skills::installer::skill_status(name, entry.version(), *agent) {
                SkillStatus::Installed { .. } => installed_count += 1,
                SkillStatus::Missing => missing_count += 1,
                SkillStatus::Drift { .. } => drift_count += 1,
            }
        }
    }
    println!("Skills Status");
    println!("=============");
    println!("Installed: {installed_count}");
    println!("Missing:   {missing_count}");
    println!("Drift:     {drift_count}");
    if drift_count > 0 || missing_count > 0 {
        println!();
        println!("Run `jarvy skills install` to install missing skills.");
    }
    0
}

fn agents_action() -> i32 {
    let agents = skills::detect_agents();
    println!("Detected AI agents:");
    if agents.is_empty() {
        println!("  (none)");
        return 0;
    }
    for a in agents {
        println!("  {} ({})", a.slug(), a.config_dir().unwrap().display());
    }
    0
}

fn resolve_target_agents(cfg: &SkillsConfig) -> Vec<SkillAgent> {
    if cfg.agents.is_empty() {
        return skills::detect_agents();
    }
    cfg.agents
        .iter()
        .filter_map(|slug| SkillAgent::from_slug(slug))
        .collect()
}

fn prepare_library_sources(cfg: &SkillsConfig) {
    crate::library_registry::sync_all("skills", "", &cfg.library_sources, cfg.origin);
}
