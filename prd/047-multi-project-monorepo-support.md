# PRD-047: Multi-Project/Monorepo Support

## Overview

Enable Jarvy to handle monorepos and multi-project workspaces where different directories have different tool requirements, providing unified setup while respecting per-project configurations.

## Problem Statement

Modern development often involves monorepos or workspaces with multiple projects:

- Frontend (Node.js) and backend (Go) in same repo
- Multiple microservices with different tech stacks
- Shared libraries in different languages
- Infrastructure code alongside application code

Currently, Jarvy assumes a single `jarvy.toml` at the root, which doesn't handle projects with diverse requirements across directories.

## Evidence

- Monorepos increasingly common (Google, Meta, Microsoft patterns)
- "Which directory do I need to be in?" confusion
- Manual tool switching when moving between projects
- Conflicting tool versions between projects
- Duplicated tool definitions across repos in multi-repo setups

## Requirements

### Functional Requirements

1. **Config discovery**: Find and merge configs from root to subdirectories
2. **Workspace support**: Define workspace members in root config
3. **Per-project tools**: Override tools per project directory
4. **Shared tools**: Inherit common tools from root
5. **Project selection**: Setup specific projects only
6. **Context awareness**: Detect current directory context

### Non-Functional Requirements

1. **Backwards compatible**: Single-project repos work unchanged
2. **Fast discovery**: Config scanning <100ms
3. **Clear precedence**: Intuitive override behavior
4. **Minimal duplication**: Share configs where possible
5. **IDE friendly**: Work with standard project structures

## Non-Goals

- Cross-repository configuration sharing
- Remote workspace definitions
- Build system integration (Bazel, Buck, Nx)
- Task orchestration across projects
- Workspace-aware version managers (nvm per-directory)

## Feature Specifications

### 1. Workspace Configuration

```toml
# Root jarvy.toml

# Declare this as a workspace
[workspace]
# Member projects (directories with their own jarvy.toml)
members = [
    "apps/web",
    "apps/api",
    "packages/*",        # Glob patterns supported
    "infrastructure",
]

# Default tools for all workspace members
default_members = true  # Include tools below in all members

# Shared tools (available to all workspace members)
[provisioner]
git = "latest"
docker = "latest"
kubectl = "latest"

# Shared hooks
[hooks.docker]
post_install = "docker compose pull"
```

### 2. Member Project Configuration

```toml
# apps/web/jarvy.toml

# Inherit from workspace root
workspace = ".."  # Or: workspace = true (auto-detect)

# Project-specific tools (added to workspace tools)
[provisioner]
node = "20"
bun = "latest"
playwright = "latest"

# Override workspace tool version
[provisioner]
docker = "24.0.0"  # Use specific version instead of latest

# Project-specific hooks
[hooks.node]
post_install = "npm install"
```

```toml
# apps/api/jarvy.toml

workspace = true

[provisioner]
go = "1.21"
air = "latest"      # Go hot reload
golangci-lint = "latest"

[hooks.go]
post_install = "go mod download"
```

### 3. CLI Commands

```bash
# Setup entire workspace (all members)
jarvy setup

# Output:
# Workspace: /path/to/monorepo
# Members: apps/web, apps/api, packages/shared, infrastructure
#
# Setting up workspace...
#
# [Shared Tools]
#   ✓ git 2.43.0
#   ✓ docker 24.0.7
#   ✓ kubectl 1.29.0
#
# [apps/web]
#   ✓ node 20.10.0
#   ✓ bun 1.0.20
#   ✓ playwright 1.40.0
#   Running post_install hook...
#
# [apps/api]
#   ✓ go 1.21.5
#   ✓ air 1.49.0
#   ✓ golangci-lint 1.55.0
#   Running post_install hook...
#
# Workspace setup complete!

# Setup specific member only
jarvy setup --project apps/web
jarvy setup -p api  # Short name matching

# Setup current directory's project
cd apps/web
jarvy setup  # Detects context, sets up apps/web

# List workspace members
jarvy workspace list

# Output:
# Workspace Members
# =================
#
# Project          Path              Tools
# ─────────────────────────────────────────
# web              apps/web          node, bun, playwright
# api              apps/api          go, air, golangci-lint
# shared           packages/shared   rust, cargo-watch
# infrastructure   infrastructure    terraform, terragrunt

# Show merged config for a project
jarvy workspace show apps/web

# Output:
# Project: apps/web
# Config: apps/web/jarvy.toml
# Workspace: /path/to/monorepo
#
# Inherited Tools (from workspace):
#   git        latest
#   docker     latest → 24.0.0 (overridden)
#   kubectl    latest
#
# Project Tools:
#   node       20
#   bun        latest
#   playwright latest

# Validate workspace configuration
jarvy workspace validate

# Output:
# Validating workspace...
#   ✓ Root config valid
#   ✓ apps/web config valid
#   ✓ apps/api config valid
#   ⚠ packages/shared missing jarvy.toml (using workspace defaults)
#   ✓ infrastructure config valid
#
# Workspace valid with 1 warning
```

### 4. Directory Context Detection

```bash
# Automatic project detection
cd /path/to/monorepo/apps/web
jarvy setup
# → Detects apps/web context, sets up only that project

cd /path/to/monorepo
jarvy setup
# → Sets up entire workspace

# Show current context
jarvy context

# Output:
# Current Context
# ===============
# Directory: /path/to/monorepo/apps/web
# Workspace: /path/to/monorepo
# Project: apps/web
# Config: apps/web/jarvy.toml
# Tools: node, bun, playwright (+ 3 inherited)

# Override context detection
jarvy setup --workspace  # Force workspace mode
jarvy setup --no-workspace  # Force single-project mode
```

### 5. Config Inheritance

```toml
# Root jarvy.toml
[workspace]
members = ["apps/*", "packages/*"]

[provisioner]
git = "latest"
node = "18"      # Default Node version
docker = "latest"

[roles.base]
tools = ["git", "docker"]

# apps/web/jarvy.toml
workspace = true

[provisioner]
node = "20"      # Override: use Node 20
react-devtools = "latest"

# Inherited: git, docker
# Overridden: node (18 → 20)
# Added: react-devtools
```

## Technical Approach

### Module Structure

```
src/
  workspace/
    mod.rs           # Public API
    config.rs        # Workspace configuration
    discovery.rs     # Config file discovery
    resolver.rs      # Config inheritance resolution
    context.rs       # Directory context detection
    commands.rs      # CLI command handlers
```

### Configuration Types

```rust
// src/workspace/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WorkspaceConfig {
    /// Member project patterns
    #[serde(default)]
    pub members: Vec<String>,

    /// Apply workspace [provisioner] to all members
    #[serde(default = "default_true")]
    pub default_members: bool,

    /// Exclude patterns
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WorkspaceRef {
    /// Explicit path to workspace root
    Path(String),
    /// Auto-detect workspace root
    Auto(bool),
}

impl Default for WorkspaceRef {
    fn default() -> Self {
        WorkspaceRef::Auto(false)
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    pub name: String,
    pub path: PathBuf,
    pub config: Option<JarvyConfig>,
    pub resolved_tools: HashMap<String, ToolSpec>,
}
```

### Config Discovery

```rust
// src/workspace/discovery.rs
use std::path::{Path, PathBuf};
use glob::glob;

pub struct ConfigDiscovery;

impl ConfigDiscovery {
    /// Find workspace root from current directory
    pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
        let mut current = start.to_path_buf();

        loop {
            let config_path = current.join("jarvy.toml");
            if config_path.exists() {
                if let Ok(config) = JarvyConfig::from_file(&config_path) {
                    if config.workspace.is_some() {
                        return Some(current);
                    }
                }
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Find all workspace members
    pub fn find_members(workspace_root: &Path, config: &WorkspaceConfig) -> Vec<PathBuf> {
        let mut members = Vec::new();

        for pattern in &config.members {
            let full_pattern = workspace_root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob(&pattern_str).unwrap().flatten() {
                if entry.is_dir() && !Self::is_excluded(&entry, workspace_root, &config.exclude) {
                    members.push(entry);
                }
            }
        }

        members.sort();
        members.dedup();
        members
    }

    /// Find config for a specific directory
    pub fn find_config(dir: &Path) -> Option<PathBuf> {
        let config_path = dir.join("jarvy.toml");
        if config_path.exists() {
            Some(config_path)
        } else {
            None
        }
    }

    fn is_excluded(path: &Path, root: &Path, patterns: &[String]) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        for pattern in patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches_path(relative) {
                    return true;
                }
            }
        }
        false
    }
}
```

### Config Resolution

```rust
// src/workspace/resolver.rs
use std::collections::HashMap;

pub struct ConfigResolver;

impl ConfigResolver {
    /// Resolve final tool configuration for a member
    pub fn resolve_member(
        workspace_config: &JarvyConfig,
        member_config: Option<&JarvyConfig>,
        workspace_defaults: bool,
    ) -> HashMap<String, ToolSpec> {
        let mut resolved = HashMap::new();

        // Start with workspace tools if default_members is true
        if workspace_defaults {
            for (name, spec) in workspace_config.provisioner_iter() {
                resolved.insert(name.clone(), spec.clone());
            }
        }

        // Overlay member-specific tools
        if let Some(member) = member_config {
            for (name, spec) in member.provisioner_iter() {
                resolved.insert(name.clone(), spec.clone());
            }
        }

        resolved
    }

    /// Resolve hooks (member overrides workspace)
    pub fn resolve_hooks(
        workspace_config: &JarvyConfig,
        member_config: Option<&JarvyConfig>,
    ) -> HashMap<String, Hook> {
        let mut resolved = HashMap::new();

        // Workspace hooks
        for (name, hook) in &workspace_config.hooks {
            resolved.insert(name.clone(), hook.clone());
        }

        // Member hooks override
        if let Some(member) = member_config {
            for (name, hook) in &member.hooks {
                resolved.insert(name.clone(), hook.clone());
            }
        }

        resolved
    }
}
```

### Context Detection

```rust
// src/workspace/context.rs
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub workspace_root: Option<PathBuf>,
    pub current_project: Option<WorkspaceMember>,
    pub is_workspace: bool,
    pub is_project_root: bool,
}

impl WorkspaceContext {
    pub fn detect(current_dir: &Path) -> Self {
        // Try to find workspace root
        let workspace_root = ConfigDiscovery::find_workspace_root(current_dir);

        if let Some(ref root) = workspace_root {
            let workspace_config = JarvyConfig::from_file(&root.join("jarvy.toml")).ok();

            // Are we at the workspace root?
            if current_dir == root.as_path() {
                return Self {
                    workspace_root: Some(root.clone()),
                    current_project: None,
                    is_workspace: true,
                    is_project_root: false,
                };
            }

            // Find which member we're in
            if let Some(ref config) = workspace_config {
                if let Some(ws) = &config.workspace {
                    let members = ConfigDiscovery::find_members(root, ws);
                    for member_path in members {
                        if current_dir.starts_with(&member_path) {
                            let member_config = ConfigDiscovery::find_config(&member_path)
                                .and_then(|p| JarvyConfig::from_file(&p).ok());

                            return Self {
                                workspace_root: Some(root.clone()),
                                current_project: Some(WorkspaceMember {
                                    name: member_path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                    path: member_path.clone(),
                                    config: member_config,
                                    resolved_tools: HashMap::new(),
                                }),
                                is_workspace: true,
                                is_project_root: current_dir == member_path,
                            };
                        }
                    }
                }
            }
        }

        // No workspace, check for standalone config
        let has_config = current_dir.join("jarvy.toml").exists();

        Self {
            workspace_root: None,
            current_project: None,
            is_workspace: false,
            is_project_root: has_config,
        }
    }
}
```

## Implementation Steps

1. Create workspace module structure
2. Implement WorkspaceConfig parsing
3. Implement config file discovery
4. Implement member pattern matching (glob)
5. Implement config resolution (inheritance)
6. Implement context detection
7. Update setup command for workspace mode
8. Implement `workspace list` command
9. Implement `workspace show` command
10. Implement `workspace validate` command
11. Implement `--project` flag for targeted setup
12. Add context command
13. Write tests for workspace scenarios
14. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Multi-project setup commands | Multiple per project | 1 for workspace |
| Tool duplication across configs | High | Minimal |
| Context confusion | Common | Rare |
| Monorepo support satisfaction | Poor | Good |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Complex inheritance bugs | Medium | High | Clear precedence rules, validation |
| Glob pattern edge cases | Medium | Low | Comprehensive testing |
| Context detection wrong | Low | Medium | Explicit overrides available |
| Performance on large monorepos | Low | Medium | Lazy config loading, caching |
| Breaking existing configs | Low | High | Backwards compatible design |

## Dependencies

### New Dependencies
- `glob` - Pattern matching for workspace members (likely already present)

### Existing Dependencies
- `toml` - Config parsing
- `serde` - Serialization

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| Config discovery | 1 day |
| Member pattern matching | 0.5 days |
| Config resolution | 1 day |
| Context detection | 0.5 days |
| Setup command integration | 1 day |
| workspace list command | 0.5 days |
| workspace show command | 0.5 days |
| workspace validate command | 0.5 days |
| --project flag | 0.5 days |
| context command | 0.25 days |
| Testing | 1.5 days |
| Documentation | 0.5 days |
| **Total** | **8.5 days** |

## Files to Create/Modify

### New Files
- `src/workspace/mod.rs`
- `src/workspace/config.rs`
- `src/workspace/discovery.rs`
- `src/workspace/resolver.rs`
- `src/workspace/context.rs`
- `src/workspace/commands.rs`
- `tests/workspace_integration.rs`

### Modified Files
- `src/config.rs` - Add workspace and workspace_ref parsing
- `src/lib.rs` - Export workspace module
- `src/main.rs` - Add workspace subcommand
- `src/commands/setup_cmd.rs` - Add workspace and project modes
- `CLAUDE.md` - Document [workspace] section

---

*PRD-047 v1.0 | Multi-Project/Monorepo Support | Priority: Low*
