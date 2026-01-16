# PRD-029: Cross-Project Consistency

## Overview

Enable consistent tool management across multiple projects by supporting monorepos, project-local configurations, tool version scoping, and project-specific caching.

## Problem Statement

Developers working on multiple projects face version conflicts:
- Different projects need different tool versions
- No equivalent to `nvm use` or `pyenv local` in Jarvy
- Monorepos need multiple configurations
- Global tools conflict with project requirements
- Switching projects requires manual tool version changes
- No project-local cache or state

Jarvy currently operates globally, which doesn't match how developers actually work across multiple projects.

## Evidence

- Developers work on 3-5 projects simultaneously on average
- nvm, pyenv, rbenv popular due to per-project versions
- Monorepos increasingly common (Google, Meta, Microsoft)
- "Works on my machine" often due to version mismatches
- Teams request per-project tool isolation

## Requirements

### Functional Requirements

1. **Project-local configs**: `.jarvy/config.toml` in project root
2. **Monorepo support**: Multiple `jarvy.toml` files per workspace
3. **Tool scoping**: Global vs project-local tool installations
4. **Version switching**: Automatic version switch on directory change
5. **Project caches**: `.jarvy/` directory for project state

### Non-Functional Requirements

1. Directory detection adds < 10ms overhead
2. Version switching is seamless (< 500ms)
3. Backward compatible with global-only usage
4. Works with existing version managers
5. Supports nested project structures

## Non-Goals

- Full virtual environment isolation (like Docker)
- Hermetic builds (like Bazel)
- Per-file tool versions
- Language-specific version management (defer to nvm, pyenv, etc.)
- IDE/editor integration (future PRD)

## Feature Specifications

### 1. Project-Local Configuration

Project-specific settings in `.jarvy/` directory.

```
my-project/
├── .jarvy/
│   ├── config.toml      # Project-local Jarvy settings
│   ├── cache/           # Project-specific cache
│   └── versions.json    # Currently active versions
├── jarvy.toml           # Project tool requirements
└── src/
    └── ...
```

```toml
# .jarvy/config.toml
[project]
name = "my-project"
# Use project-local tool installations
local_tools = true
# Cache directory (relative to .jarvy/)
cache_dir = "cache"

[versions]
# Override global tool versions for this project
node = "18.19.0"    # Project needs LTS
python = "3.9"      # Legacy requirement

[hooks]
# Project-specific hooks that run on entering project
on_enter = '''
export PROJECT_ENV="development"
export API_URL="http://localhost:3000"
'''

on_exit = '''
unset PROJECT_ENV
unset API_URL
'''
```

```bash
# Initialize project-local config
jarvy project init

# Output:
# Initializing Jarvy project configuration...
#
# Created .jarvy/config.toml
# Created .jarvy/cache/
#
# Project configuration:
#   Local tools: Enabled
#   Cache: .jarvy/cache/
#
# Next steps:
#   1. Edit .jarvy/config.toml for project settings
#   2. Run 'jarvy setup' to install project tools
#   3. Add .jarvy/ to .gitignore (or commit config.toml)

# Show project config
jarvy project show

# Output:
# Project: my-project
# Root: /path/to/my-project
#
# Configuration:
#   Local tools: Yes
#   Cache: .jarvy/cache/ (12.4 MB)
#
# Version overrides:
#   node: 18.19.0 (global: 21.0.0)
#   python: 3.9 (global: 3.12)
#
# Status: Active

# Sync project tools
jarvy project sync
```

**Project config features:**
- Project identification
- Local tool installation flag
- Version overrides
- Project-specific hooks
- Cache management

### 2. Monorepo Support

Handle multiple configurations within a single repository.

```
monorepo/
├── jarvy.toml           # Root-level shared tools
├── .jarvy/
│   └── config.toml      # Monorepo configuration
├── packages/
│   ├── frontend/
│   │   ├── jarvy.toml   # Frontend-specific tools
│   │   └── package.json
│   ├── backend/
│   │   ├── jarvy.toml   # Backend-specific tools
│   │   └── Cargo.toml
│   └── shared/
│       └── jarvy.toml   # Shared library tools
└── tools/
    └── jarvy.toml       # DevOps tools
```

```toml
# Root jarvy.toml
[workspace]
# Define workspace members
members = [
    "packages/frontend",
    "packages/backend",
    "packages/shared",
    "tools",
]

# Tools available to all workspace members
[tools]
git = "latest"
jq = "latest"
```

```toml
# packages/frontend/jarvy.toml
[project]
name = "frontend"
# Inherit from workspace root
inherits = ["../..", "workspace"]

[tools]
node = "20"
```

```toml
# packages/backend/jarvy.toml
[project]
name = "backend"
inherits = ["workspace"]

[tools]
rust = "1.75"
docker = "latest"
```

```bash
# Setup entire workspace
jarvy workspace setup

# Output:
# Workspace: monorepo
# Members: 4 packages
#
# Resolving dependencies...
#   Root: git, jq
#   frontend: node (inherits: git, jq)
#   backend: rust, docker (inherits: git, jq)
#   shared: (inherits: git, jq)
#   tools: terraform, kubectl (inherits: git, jq)
#
# Installing workspace tools...
#   [1/7] git (shared)... ✓
#   [2/7] jq (shared)... ✓
#   [3/7] node (frontend)... ✓
#   [4/7] rust (backend)... ✓
#   [5/7] docker (backend)... ✓
#   [6/7] terraform (tools)... ✓
#   [7/7] kubectl (tools)... ✓
#
# ✓ Workspace setup complete

# Setup specific member
jarvy workspace setup frontend

# List workspace members
jarvy workspace list

# Output:
# Workspace Members
# =================
#
# Member         Path                  Tools  Status
# ──────────────────────────────────────────────────
# root           .                     2      ✓
# frontend       packages/frontend     1+2    ✓
# backend        packages/backend      2+2    ✓
# shared         packages/shared       0+2    ✓
# tools          tools                 2+2    ✓
#
# Total unique tools: 7

# Show workspace tool resolution
jarvy workspace resolve

# Output:
# Tool Resolution
# ===============
#
# git (latest)
#   Defined in: root
#   Used by: all members
#
# node (20)
#   Defined in: frontend
#   Used by: frontend
#
# rust (1.75)
#   Defined in: backend
#   Used by: backend
```

**Monorepo features:**
- Workspace member definition
- Configuration inheritance
- Shared tool deduplication
- Per-member setup
- Resolution visualization

### 3. Tool Version Scoping

Control where tools are installed and accessed.

```bash
# Install tool globally
jarvy install git --global

# Install tool locally (project-only)
jarvy install node --local

# Output:
# Installing node locally...
#
# Location: .jarvy/tools/node/20.11.0/
# Scope: Project only
#
# This tool will only be available when working in this project.
# It will be activated automatically when entering this directory.

# Show tool scope
jarvy tools scope

# Output:
# Tool Scopes
# ===========
#
# Tool        Scope     Version     Location
# ─────────────────────────────────────────────────────────
# git         global    2.43.0      /opt/homebrew/bin/git
# node        local     20.11.0     .jarvy/tools/node/
# docker      global    24.0.7      /Applications/Docker.app
# python      local     3.9.18      .jarvy/tools/python/
#
# Legend:
#   global - Available system-wide
#   local  - Available in this project only

# Convert global to local
jarvy tools localize node

# Convert local to global
jarvy tools globalize python
```

```toml
# jarvy.toml with explicit scopes
[tools]
# Explicit global installation
git = { version = "latest", scope = "global" }

# Local installation (default if .jarvy/config.toml has local_tools = true)
node = { version = "20", scope = "local" }

# Prefer local, fallback to global
rust = { version = "1.75", scope = "prefer-local" }
```

**Scoping features:**
- Global vs local installation
- Automatic scope detection
- Scope conversion commands
- Scope inheritance rules
- Clear scope visualization

### 4. Automatic Version Switching

Switch tool versions when changing directories.

```bash
# Enable shell integration for auto-switching
jarvy shell init zsh >> ~/.zshrc
source ~/.zshrc

# Now, changing directories triggers version switching:
$ cd ~/projects/legacy-app
# [jarvy] Switching to project: legacy-app
#   node: 21.0.0 -> 18.19.0
#   python: 3.12 -> 3.9

$ node --version
v18.19.0

$ cd ~/projects/modern-app
# [jarvy] Switching to project: modern-app
#   node: 18.19.0 -> 21.0.0

$ node --version
v21.0.0

# Disable switching notification
jarvy config set shell.quiet true

# Manual version switch (without cd)
jarvy use node 18.19.0

# Output:
# Switching node version...
#   21.0.0 -> 18.19.0
#
# ✓ Now using node 18.19.0
#
# Note: This is temporary. Version will reset on directory change.

# Show current active versions
jarvy use --show

# Output:
# Active Versions (project: legacy-app)
# =====================================
#
# Tool        Active      Global      Source
# ───────────────────────────────────────────────
# node        18.19.0     21.0.0      project
# python      3.9.18      3.12.0      project
# rust        1.75.0      1.75.0      global
# git         2.43.0      2.43.0      global
```

```zsh
# Shell integration (added to .zshrc)
eval "$(jarvy shell hook zsh)"

# This sets up:
# - PROMPT_COMMAND/precmd to detect directory changes
# - PATH manipulation for project-local tools
# - Environment variable setup from project config
```

**Auto-switching features:**
- Shell hook for directory detection
- Automatic PATH manipulation
- Environment variable setup
- Quiet mode option
- Manual override command

### 5. Project Caches

Project-specific cache and state management.

```
.jarvy/
├── config.toml           # Project configuration
├── cache/
│   ├── downloads/        # Downloaded packages
│   ├── builds/           # Compiled tools
│   └── metadata/         # Tool metadata cache
├── tools/                # Locally installed tools
│   ├── node/
│   │   └── 18.19.0/
│   └── python/
│       └── 3.9.18/
├── versions.json         # Current version state
└── history.json          # Project version history
```

```bash
# View project cache status
jarvy project cache

# Output:
# Project Cache: legacy-app
# =========================
#
# Location: .jarvy/cache/
#
# Category        Size        Items
# ─────────────────────────────────
# Downloads       45.2 MB     3
# Builds          128.4 MB    2
# Metadata        234 KB      15
#
# Total: 173.8 MB

# Clean project cache
jarvy project cache clean

# Output:
# Cleaning project cache...
#
# Removed:
#   Downloads: 45.2 MB (3 items)
#   Metadata: 234 KB (15 items)
#
# Preserved:
#   Builds: 128.4 MB (required for installed tools)
#
# Freed: 45.4 MB

# Warm project cache
jarvy project cache warm

# Export project environment
jarvy project export

# Output:
# Exporting project environment...
#
# ✓ Created: legacy-app-environment.tar.gz
#   Size: 189.2 MB
#   Tools: 4
#   Config: Included
#
# Import on another machine:
#   jarvy project import legacy-app-environment.tar.gz
```

**Project cache features:**
- Isolated from global cache
- Per-project downloads
- Local tool installations
- Version state tracking
- Export/import capability

## Acceptance Criteria

### Project-Local Configuration
- [ ] `.jarvy/config.toml` detected in project
- [ ] `jarvy project init` creates structure
- [ ] `jarvy project show` displays config
- [ ] Version overrides work
- [ ] Project hooks execute
- [ ] Cache isolation works

### Monorepo Support
- [ ] `[workspace]` section defines members
- [ ] `inherits` field works
- [ ] `jarvy workspace setup` installs all
- [ ] `jarvy workspace list` shows members
- [ ] `jarvy workspace resolve` shows resolution
- [ ] Shared tools not duplicated

### Tool Version Scoping
- [ ] `--global` and `--local` flags work
- [ ] `jarvy tools scope` shows scopes
- [ ] `jarvy tools localize` converts scope
- [ ] Local tools stored in `.jarvy/tools/`
- [ ] Scope preference in jarvy.toml works

### Automatic Version Switching
- [ ] `jarvy shell init` generates hook
- [ ] Directory change triggers switch
- [ ] PATH updated correctly
- [ ] Environment variables set
- [ ] `jarvy use` for manual switch
- [ ] `jarvy use --show` displays active

### Project Caches
- [ ] `.jarvy/cache/` created per project
- [ ] `jarvy project cache` shows status
- [ ] `jarvy project cache clean` works
- [ ] `jarvy project export` creates bundle
- [ ] `jarvy project import` restores
- [ ] Cache isolated from global

## Technical Approach

### Module Structure

```
src/
  project/
    mod.rs              # Project management
    detect.rs           # Project detection
    config.rs           # Project configuration
    workspace.rs        # Monorepo workspace
    scope.rs            # Tool scoping
    switch.rs           # Version switching
    cache.rs            # Project cache
  shell/
    mod.rs              # Shell integration
    hook.rs             # Shell hooks
    zsh.rs              # Zsh-specific
    bash.rs             # Bash-specific
    fish.rs             # Fish-specific
```

### Project Detection

```rust
// src/project/detect.rs
use std::path::{Path, PathBuf};

pub struct ProjectDetector;

impl ProjectDetector {
    pub fn find_project_root(start: &Path) -> Option<ProjectRoot> {
        let mut current = start.to_path_buf();

        loop {
            // Check for jarvy project markers
            if current.join("jarvy.toml").exists() {
                return Some(ProjectRoot {
                    path: current.clone(),
                    has_local_config: current.join(".jarvy/config.toml").exists(),
                    is_workspace: self.is_workspace(&current),
                });
            }

            // Check for workspace member (has jarvy.toml but part of workspace)
            if current.join(".jarvy/config.toml").exists() {
                return Some(ProjectRoot {
                    path: current.clone(),
                    has_local_config: true,
                    is_workspace: false,
                });
            }

            // Move up
            if !current.pop() {
                return None;
            }
        }
    }

    fn is_workspace(&self, path: &Path) -> bool {
        if let Ok(content) = std::fs::read_to_string(path.join("jarvy.toml")) {
            content.contains("[workspace]")
        } else {
            false
        }
    }
}
```

### Shell Hook Generation

```rust
// src/shell/hook.rs
pub fn generate_zsh_hook() -> String {
    r#"
# Jarvy shell integration
_jarvy_hook() {
    local project_root
    project_root="$(jarvy project detect --quiet 2>/dev/null)"

    if [[ -n "$project_root" && "$project_root" != "$JARVY_PROJECT" ]]; then
        eval "$(jarvy project enter "$project_root")"
        export JARVY_PROJECT="$project_root"
    elif [[ -z "$project_root" && -n "$JARVY_PROJECT" ]]; then
        eval "$(jarvy project exit)"
        unset JARVY_PROJECT
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd _jarvy_hook
_jarvy_hook  # Run on shell init
"#.to_string()
}
```

### Workspace Resolution

```rust
// src/project/workspace.rs
pub struct WorkspaceResolver {
    root: PathBuf,
    members: Vec<WorkspaceMember>,
}

impl WorkspaceResolver {
    pub fn resolve(&self) -> Result<ResolvedWorkspace, Error> {
        let root_config = self.load_config(&self.root)?;
        let mut all_tools = root_config.tools.clone();

        for member in &self.members {
            let member_config = self.load_config(&member.path)?;

            // Apply inheritance
            let inherited = self.apply_inheritance(&member_config, &root_config)?;

            // Merge tools (member overrides root)
            for (name, spec) in inherited.tools {
                all_tools.entry(name).or_insert(spec);
            }
        }

        Ok(ResolvedWorkspace {
            root_tools: root_config.tools,
            member_tools: self.members.iter().map(|m| {
                (m.name.clone(), self.load_config(&m.path).unwrap().tools)
            }).collect(),
            all_tools,
        })
    }
}
```

## Implementation Steps

1. Create project module structure
2. Implement project detection
3. Build project configuration
4. Add workspace support
5. Implement tool scoping
6. Create shell hooks
7. Implement version switching
8. Build project cache
9. Add export/import
10. Write shell-specific hooks
11. Add workspace commands
12. Write unit tests
13. Write integration tests
14. Update documentation

## Dependencies

- No new dependencies required
- Uses existing shell integration patterns

## Effort Estimate

| Task | Effort |
|------|--------|
| Project module structure | 0.5 days |
| Project detection | 1.5 days |
| Project configuration | 1.5 days |
| Workspace support | 2.5 days |
| Tool scoping | 2 days |
| Shell hooks | 2 days |
| Version switching | 2 days |
| Project cache | 1.5 days |
| Export/import | 1 day |
| Shell-specific hooks | 1.5 days |
| Workspace commands | 1 day |
| Testing | 3 days |
| Documentation | 1 day |
| **Total** | **21 days** |

## Files to Create/Modify

### New Files
- `src/project/mod.rs`
- `src/project/detect.rs`
- `src/project/config.rs`
- `src/project/workspace.rs`
- `src/project/scope.rs`
- `src/project/switch.rs`
- `src/project/cache.rs`
- `src/shell/mod.rs`
- `src/shell/hook.rs`
- `src/shell/zsh.rs`
- `src/shell/bash.rs`
- `src/shell/fish.rs`
- `tests/project_integration.rs`
- `tests/workspace_integration.rs`

### Modified Files
- `src/main.rs` - Add project/workspace commands
- `src/commands/mod.rs` - Export new modules
- `CLAUDE.md` - Document project features

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Project-local configs | None | Full support |
| Monorepo support | None | Workspace |
| Tool scoping | Global only | Global + Local |
| Auto version switch | None | Shell hooks |
| Project isolation | None | Complete |
| Multi-project workflow | Manual | Automatic |

## Risks

1. **Shell compatibility**: Different shells have different hook mechanisms
   - Mitigation: Support major shells, document others

2. **Performance**: Directory detection on every prompt
   - Mitigation: Cache results, minimal checks

3. **Complexity**: Project hierarchy can be confusing
   - Mitigation: Clear status commands, visualization

4. **Disk usage**: Local tools use more space
   - Mitigation: Deduplication, cleanup commands

5. **Version manager conflicts**: May conflict with nvm, pyenv
   - Mitigation: Integration options, clear precedence
