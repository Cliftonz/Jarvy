//! `jarvy library` CLI handler (PRD-054 phase 6).
//!
//! Four read-mostly subcommands over the library-registry cache:
//!
//! - `list`  — every cached library + its item counts
//! - `show`  — items inside one cached library (by URL)
//! - `clean` — wipe `~/.jarvy/library.d/` (and the process cache)
//! - `sync`  — force-refresh every `[<subsystem>.library_sources]` entry
//!
//! `sync` is the only mutating subcommand. Cosign signature
//! enforcement (PRD-054 phase 5) runs inline during the sync — if
//! `require_signature = true` and the manifest's signature companions
//! fail to verify, the sync surfaces a `LibraryError::SignatureRejected`
//! / `SignatureCompanionsMissing` / `CosignMissing` and the cache is
//! left untouched.

use crate::cli::LibraryAction;
use crate::config::Config;
use crate::library_registry;

pub fn run_library(action: &LibraryAction) -> i32 {
    match action {
        LibraryAction::List { output_format } => list(output_format),
        LibraryAction::Show { url, output_format } => show(url, output_format),
        LibraryAction::Clean {
            dry_run,
            output_format,
        } => clean(*dry_run, output_format),
        LibraryAction::Sync {
            file,
            output_format,
        } => sync(file, output_format),
    }
}

fn list(output_format: &str) -> i32 {
    let libs = library_registry::list_cached();

    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "count": libs.len(),
                "libraries": libs,
            }))
            .unwrap_or_else(|_| "{}".into())
        );
        return 0;
    }

    if libs.is_empty() {
        println!("No libraries cached.");
        println!(
            "Add a `[<subsystem>.library_sources]` block to jarvy.toml and run `jarvy library sync`."
        );
        return 0;
    }

    println!("Cached libraries ({}):", libs.len());
    for lib in &libs {
        println!();
        println!("  {}", lib.url);
        if !lib.publisher.is_empty() {
            println!("    publisher: {}", lib.publisher);
        }
        if !lib.description.is_empty() {
            println!("    description: {}", lib.description);
        }
        println!(
            "    items: {} ai_hook, {} mcp_server, {} skill",
            lib.ai_hook_count, lib.mcp_server_count, lib.skill_count
        );
    }
    0
}

fn show(url: &str, output_format: &str) -> i32 {
    match library_registry::get_cached(url) {
        Some((cached_url, manifest)) => {
            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "url": cached_url,
                        "publisher": manifest.publisher,
                        "description": manifest.description,
                        "schema_version": manifest.schema_version,
                        "generated_at": manifest.generated_at,
                        "items": &manifest.items,
                    }))
                    .unwrap_or_else(|_| "{}".into())
                );
                return 0;
            }
            println!("Library: {cached_url}");
            if !manifest.publisher.is_empty() {
                println!("Publisher: {}", manifest.publisher);
            }
            if !manifest.description.is_empty() {
                println!("Description: {}", manifest.description);
            }
            println!();
            let (hooks, servers, skills) = bucket_items(&manifest.items);
            if !hooks.is_empty() {
                println!("AI hooks ({}):", hooks.len());
                for name in &hooks {
                    println!("  - {name}");
                }
                println!();
            }
            if !servers.is_empty() {
                println!("MCP servers ({}):", servers.len());
                for name in &servers {
                    println!("  - {name}");
                }
                println!();
            }
            if !skills.is_empty() {
                println!("Skills ({}):", skills.len());
                for name in &skills {
                    println!("  - {name}");
                }
            }
            0
        }
        None => {
            if output_format == "json" {
                println!("{}", serde_json::json!({"status": "not_found", "url": url}));
            } else {
                eprintln!("No cached library at URL: {url}");
                eprintln!("Run `jarvy library list` to see available URLs.");
            }
            crate::error_codes::CONFIG_ERROR
        }
    }
}

fn bucket_items(
    items: &[library_registry::LibraryItem],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut hooks = Vec::new();
    let mut servers = Vec::new();
    let mut skills = Vec::new();
    for item in items {
        match item {
            library_registry::LibraryItem::AiHook(h) => hooks.push(h.name.clone()),
            library_registry::LibraryItem::McpServer(s) => servers.push(s.name.clone()),
            library_registry::LibraryItem::Skill(s) => skills.push(s.name.clone()),
        }
    }
    hooks.sort();
    servers.sort();
    skills.sort();
    (hooks, servers, skills)
}

fn clean(dry_run: bool, output_format: &str) -> i32 {
    let root = match library_registry::cache::cache_root() {
        Ok(p) => p,
        Err(e) => {
            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::json!({"status": "error", "error": e.to_string()})
                );
            } else {
                eprintln!("Failed to resolve library cache root: {e}");
            }
            return crate::error_codes::CONFIG_ERROR;
        }
    };

    if dry_run {
        let entries: Vec<String> = std::fs::read_dir(&root)
            .ok()
            .map(|it| {
                it.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        if output_format == "json" {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "dry_run": true,
                    "cache_root": root.display().to_string(),
                    "would_remove_count": entries.len(),
                    "would_remove": entries,
                }))
                .unwrap_or_else(|_| "{}".into())
            );
        } else if entries.is_empty() {
            println!("Library cache is already empty: {}", root.display());
        } else {
            println!(
                "Would remove {} entries from {}:",
                entries.len(),
                root.display()
            );
            for e in &entries {
                println!("  {e}");
            }
        }
        return 0;
    }

    match library_registry::clear_disk_cache() {
        Ok((removed, bytes)) => {
            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "removed_count": removed,
                        "removed_bytes": bytes,
                        "cache_root": root.display().to_string(),
                    }))
                    .unwrap_or_else(|_| "{}".into())
                );
            } else if removed == 0 {
                println!("Library cache already empty.");
            } else {
                println!(
                    "Removed {removed} cached libraries ({}).",
                    crate::logging::format_size(bytes)
                );
            }
            0
        }
        Err(e) => {
            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::json!({"status": "error", "error": e.to_string()})
                );
            } else {
                eprintln!("Failed to clean library cache: {e}");
            }
            crate::error_codes::CONFIG_ERROR
        }
    }
}

fn sync(file: &str, output_format: &str) -> i32 {
    let config = Config::new(file);
    // Pull every library_sources block declared anywhere in jarvy.toml.
    // We re-route through `library_registry::sync_all` per subsystem so
    // the trust-gate semantics (remote-fetched configs CAN'T declare
    // library_sources) are honored identically to the setup path.
    let mut reports = Vec::new();

    if let Some(ai) = config.ai_hooks.as_ref() {
        library_registry::sync_all(
            "ai_hooks",
            "(check `jarvy library list`)",
            &ai.library_sources,
            ai.origin,
        );
        for src in &ai.library_sources {
            reports.push(serde_json::json!({"consumer": "ai_hooks", "url": src.url}));
        }
    }
    if let Some(mcp) = config.mcp_register.as_ref() {
        library_registry::sync_all(
            "mcp_register",
            "(check `jarvy library list`)",
            &mcp.library_sources,
            mcp.origin,
        );
        for src in &mcp.library_sources {
            reports.push(serde_json::json!({"consumer": "mcp_register", "url": src.url}));
        }
    }
    if let Some(sk) = config.skills.as_ref() {
        library_registry::sync_all("skills", "", &sk.library_sources, sk.origin);
        for src in &sk.library_sources {
            reports.push(serde_json::json!({"consumer": "skills", "url": src.url}));
        }
    }

    let total = reports.len();
    let cached = library_registry::list_cached();
    if output_format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "sources_attempted": total,
                "libraries_cached": cached.len(),
                "sources": reports,
            }))
            .unwrap_or_else(|_| "{}".into())
        );
        return 0;
    }
    if total == 0 {
        println!("No `library_sources` declared in {file}.");
        println!(
            "Add `[[<subsystem>.library_sources]]` blocks under [ai_hooks], [mcp_register], or [skills]."
        );
    } else {
        println!(
            "Synced {total} library source(s); {} libraries cached.",
            cached.len()
        );
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial(jarvy_home_env)]
    fn list_returns_zero_when_cache_empty() {
        // SAFETY: scoped JARVY_HOME so we don't touch the dev's real ~/.jarvy
        #[allow(unsafe_code)]
        unsafe {
            let tmp = tempfile::tempdir().unwrap();
            std::env::set_var("JARVY_HOME", tmp.path());
            library_registry::clear_cache();
            assert_eq!(list("pretty"), 0);
            assert_eq!(list("json"), 0);
            std::env::remove_var("JARVY_HOME");
        }
    }

    #[test]
    #[serial(jarvy_home_env)]
    fn show_unknown_url_returns_config_error() {
        #[allow(unsafe_code)]
        unsafe {
            let tmp = tempfile::tempdir().unwrap();
            std::env::set_var("JARVY_HOME", tmp.path());
            library_registry::clear_cache();
            assert_eq!(
                show("https://no.such/url.json", "pretty"),
                crate::error_codes::CONFIG_ERROR
            );
            std::env::remove_var("JARVY_HOME");
        }
    }

    #[test]
    #[serial(jarvy_home_env)]
    fn clean_dry_run_returns_zero_on_empty_cache() {
        #[allow(unsafe_code)]
        unsafe {
            let tmp = tempfile::tempdir().unwrap();
            std::env::set_var("JARVY_HOME", tmp.path());
            assert_eq!(clean(true, "pretty"), 0);
            std::env::remove_var("JARVY_HOME");
        }
    }

    #[test]
    #[serial(jarvy_home_env)]
    fn clean_wipes_disk_cache() {
        #[allow(unsafe_code)]
        unsafe {
            let tmp = tempfile::tempdir().unwrap();
            std::env::set_var("JARVY_HOME", tmp.path());
            // Materialize the cache root manually so there's something
            // to delete.
            let root = library_registry::cache::cache_root().unwrap();
            std::fs::create_dir_all(root.join("aaaa")).unwrap();
            std::fs::write(root.join("aaaa/manifest.json"), "{}").unwrap();

            let exit = clean(false, "pretty");
            assert_eq!(exit, 0);
            assert!(std::fs::read_dir(&root).unwrap().next().is_none());
            std::env::remove_var("JARVY_HOME");
        }
    }
}
