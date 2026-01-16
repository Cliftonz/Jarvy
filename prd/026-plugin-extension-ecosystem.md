# PRD-026: Plugin & Extension Ecosystem

## Overview

Create an extensible plugin system that allows third-party tool definitions, custom hook libraries, and community-contributed configurations, enabling Jarvy to support tools beyond the built-in registry.

## Problem Statement

Jarvy's current architecture limits extensibility:
- Only ~150 built-in tools; can't cover everyone's needs
- Adding tools requires code changes and releases
- No path for proprietary/internal tools without forking
- Community contributions limited to pull requests
- Reusable hook scripts not shareable
- No way to share jarvy.toml snippets or templates

Users with specialized tools or internal software cannot use Jarvy without either contributing to the core project or forking it.

## Evidence

- Requests for tools not in registry ("Can you add X?")
- Enterprises have internal tools that need provisioning
- Hook scripts duplicated across projects
- Popular tools (Homebrew, npm) have extension ecosystems
- Developer tool adoption correlates with extensibility

## Requirements

### Functional Requirements

1. **Custom tool definitions**: User-defined tool specs
2. **Plugin registry**: Discover and install community plugins
3. **Hook libraries**: Reusable hook script packages
4. **Template sharing**: Community jarvy.toml snippets
5. **Custom package manager support**: Extension points for new managers
6. **Local plugin development**: Easy plugin creation workflow

### Non-Functional Requirements

1. Plugins are sandboxed (limited filesystem/network access)
2. Plugin loading adds < 50ms to startup
3. Backward compatible with existing tool definitions
4. Clear security model for third-party code
5. Works offline with installed plugins

## Non-Goals

- Paid plugin marketplace
- Plugin analytics or metrics
- Automatic plugin updates
- Plugin signing/verification (future PRD)
- GUI plugin manager

## Feature Specifications

### 1. Custom Tool Definitions

Allow users to define tools outside the built-in registry.

```toml
# ~/.jarvy/plugins/tools/mytool.toml
[tool]
name = "mytool"
command = "mytool"
description = "Internal company tool"
homepage = "https://internal.company.com/mytool"

[tool.macos]
method = "script"
script = '''
curl -sL https://internal.company.com/install.sh | bash
'''

[tool.macos.verify]
command = "mytool --version"
pattern = "^mytool (\\d+\\.\\d+\\.\\d+)"

[tool.linux]
method = "apt"
package = "company-mytool"
repository = "https://apt.company.com"

[tool.windows]
method = "script"
script = '''
Invoke-WebRequest https://internal.company.com/mytool.msi -OutFile mytool.msi
Start-Process msiexec.exe -Wait -ArgumentList '/i mytool.msi /quiet'
'''

# Optional: Default hook
[tool.hook]
description = "Configure mytool credentials"
script = '''
mytool config set api-key "$MYTOOL_API_KEY"
'''
```

```bash
# Install custom tool definition
jarvy plugin add-tool ./mytool.toml

# Output:
# Adding custom tool definition...
#   Name: mytool
#   Platforms: macOS, Linux, Windows
#   Location: ~/.jarvy/plugins/tools/mytool.toml
#
# ✓ Tool 'mytool' is now available
#
# Use in jarvy.toml:
#   [tools]
#   mytool = "latest"

# List custom tools
jarvy plugin list-tools

# Output:
# Custom Tool Definitions
# =======================
#
# mytool
#   Source: ~/.jarvy/plugins/tools/mytool.toml
#   Platforms: macOS, Linux, Windows
#   Has hook: Yes
#
# company-cli
#   Source: ~/.jarvy/plugins/tools/company-cli.toml
#   Platforms: macOS, Linux
#   Has hook: No

# Remove custom tool
jarvy plugin remove-tool mytool

# Validate custom tool definition
jarvy plugin validate-tool ./mytool.toml
```

**Tool definition format:**
- Name, command, description, homepage
- Platform-specific install methods
- Version detection pattern
- Optional default hook
- Dependencies on other tools

### 2. Plugin Registry

Discover and install community plugins.

```bash
# Search the plugin registry
jarvy plugin search kubernetes

# Output:
# Plugins matching "kubernetes"
# =============================
#
# jarvy-k8s-tools (by k8s-community)
#   Kubernetes ecosystem tools (k9s, kubectx, kubens, stern, etc.)
#   Tools: 12 | Downloads: 5,234 | ★ 4.8
#   Install: jarvy plugin install jarvy-k8s-tools
#
# jarvy-helm-extras (by helm-community)
#   Additional Helm tools and plugins
#   Tools: 5 | Downloads: 1,892 | ★ 4.5
#   Install: jarvy plugin install jarvy-helm-extras
#
# jarvy-istio (by istio-contrib)
#   Istio service mesh tools
#   Tools: 3 | Downloads: 743 | ★ 4.2
#   Install: jarvy plugin install jarvy-istio

# Install a plugin
jarvy plugin install jarvy-k8s-tools

# Output:
# Installing plugin: jarvy-k8s-tools
#
# Downloading from registry...
# Verifying integrity... ✓
#
# Tools included:
#   k9s, kubectx, kubens, stern, popeye, kubeval,
#   kustomize, kubeseal, kube-score, kubespy, kubecolor, kubetail
#
# Hooks included:
#   kubectx-completion, kubens-completion
#
# ✓ Plugin installed successfully
#
# New tools available. Use in jarvy.toml:
#   [tools]
#   k9s = "latest"
#   kubectx = "latest"

# List installed plugins
jarvy plugin list

# Output:
# Installed Plugins
# =================
#
# jarvy-k8s-tools (v1.2.0)
#   Author: k8s-community
#   Tools: 12
#   Hooks: 2
#   Installed: 2024-01-15
#
# jarvy-rust-extras (v0.8.0)
#   Author: rust-community
#   Tools: 5
#   Hooks: 1
#   Installed: 2024-01-10

# Update plugins
jarvy plugin update

# Output:
# Checking for updates...
#
# jarvy-k8s-tools: 1.2.0 -> 1.3.0 (update available)
# jarvy-rust-extras: 0.8.0 (up to date)
#
# ? Update jarvy-k8s-tools? (Y/n)
#
# Updating jarvy-k8s-tools...
# ✓ Updated to v1.3.0

# Remove a plugin
jarvy plugin remove jarvy-k8s-tools
```

**Registry features:**
- Searchable plugin directory
- Plugin metadata (author, description, downloads)
- Version management
- Integrity verification
- Update notifications

### 3. Hook Libraries

Reusable hook script packages.

```toml
# ~/.jarvy/plugins/hooks/git-hooks.toml
[hook-library]
name = "git-hooks"
description = "Common git configuration hooks"
author = "jarvy-community"
version = "1.0.0"

[hooks.git-aliases]
description = "Set up common git aliases"
platforms = ["macos", "linux"]
script = '''
git config --global alias.co checkout
git config --global alias.br branch
git config --global alias.ci commit
git config --global alias.st status
git config --global alias.lg "log --oneline --graph --all"
'''

[hooks.git-config]
description = "Set up git global config"
platforms = ["macos", "linux", "windows"]
script = '''
git config --global core.autocrlf input
git config --global pull.rebase true
git config --global init.defaultBranch main
'''

[hooks.git-gpg]
description = "Configure GPG signing for git"
platforms = ["macos", "linux"]
requires = ["gpg"]
script = '''
if [ -n "$GPG_KEY_ID" ]; then
    git config --global user.signingkey "$GPG_KEY_ID"
    git config --global commit.gpgsign true
fi
'''
```

```bash
# Install hook library
jarvy plugin install-hooks git-hooks

# Output:
# Installing hook library: git-hooks
#
# Hooks included:
#   git-aliases - Set up common git aliases
#   git-config - Set up git global config
#   git-gpg - Configure GPG signing for git
#
# ✓ Hook library installed
#
# Use in jarvy.toml:
#   [hooks.git]
#   use = "git-hooks:git-aliases"

# Use hook in jarvy.toml
```

```toml
# jarvy.toml
[tools]
git = "latest"

[hooks.git]
# Use hook from library
use = "git-hooks:git-aliases"

[hooks.git-config]
# Use another hook from library
use = "git-hooks:git-config"

[hooks.custom]
# Still can use inline scripts
script = "echo 'Custom hook'"
```

```bash
# List available hooks from installed libraries
jarvy plugin list-hooks

# Output:
# Available Hooks
# ===============
#
# From: git-hooks (v1.0.0)
#   git-aliases    Set up common git aliases
#   git-config     Set up git global config
#   git-gpg        Configure GPG signing for git
#
# From: shell-hooks (v2.1.0)
#   zsh-completion  Configure zsh completions
#   bash-aliases    Common bash aliases
#   prompt-config   Shell prompt configuration

# Show hook details
jarvy plugin show-hook git-hooks:git-aliases

# Output:
# Hook: git-aliases
# Library: git-hooks
#
# Description: Set up common git aliases
# Platforms: macOS, Linux
# Requires: git
#
# Script:
#   git config --global alias.co checkout
#   git config --global alias.br branch
#   ...
```

**Hook library features:**
- Named, reusable hook scripts
- Platform-specific hooks
- Dependencies on tools
- Environment variable support
- Composition (use multiple hooks)

### 4. Template Sharing

Community-contributed jarvy.toml snippets and templates.

```bash
# Browse community templates
jarvy templates --community

# Output:
# Community Templates
# ===================
#
# Frontend:
#   react-typescript (by frontend-guild)
#     React + TypeScript + testing tools
#     Tools: 15 | ★ 4.9 | Downloads: 12,453
#
#   vue-enterprise (by vue-community)
#     Enterprise Vue.js stack
#     Tools: 18 | ★ 4.7 | Downloads: 8,921
#
# Backend:
#   go-microservices (by go-community)
#     Go microservices development
#     Tools: 14 | ★ 4.8 | Downloads: 6,234
#
#   rust-fullstack (by rust-community)
#     Full-stack Rust development
#     Tools: 12 | ★ 4.6 | Downloads: 3,892

# Install community template
jarvy templates install react-typescript

# Output:
# Installing template: react-typescript
#
# Dependencies:
#   Plugin: jarvy-frontend-tools (installing...)
#
# Template installed to ~/.jarvy/templates/react-typescript/
#
# Use it:
#   jarvy init --template react-typescript

# Publish your own template
jarvy templates publish ./my-template/

# Output:
# Publishing template...
#
# Template: my-awesome-stack
# Version: 1.0.0
# Author: your-username
#
# Validation:
#   ✓ jarvy.toml valid
#   ✓ README.md present
#   ✓ No sensitive data detected
#
# ? Publish to jarvy.dev registry? (Y/n)
#
# ✓ Published: my-awesome-stack v1.0.0
#   URL: https://jarvy.dev/templates/my-awesome-stack
```

**Template sharing features:**
- Community template registry
- Template versioning
- Dependency on plugins
- Publishing workflow
- Template ratings/downloads

### 5. Custom Package Manager Support

Extension points for new package managers.

```toml
# ~/.jarvy/plugins/managers/company-pm.toml
[package-manager]
name = "company-pm"
description = "Company internal package manager"
platforms = ["macos", "linux"]

[package-manager.detect]
# How to detect if this manager is available
command = "company-pm --version"
pattern = "^company-pm (\\d+\\.\\d+)"

[package-manager.install]
# Command template for installing packages
command = "company-pm install {package} --version {version}"
# Or script for complex installations
script = '''
company-pm login --token "$COMPANY_PM_TOKEN"
company-pm install {package}=={version}
'''

[package-manager.uninstall]
command = "company-pm uninstall {package}"

[package-manager.upgrade]
command = "company-pm upgrade {package}"

[package-manager.list]
command = "company-pm list --json"
parse = "json"  # Output format for parsing
```

```bash
# Register custom package manager
jarvy plugin add-manager ./company-pm.toml

# Output:
# Adding custom package manager...
#   Name: company-pm
#   Platforms: macOS, Linux
#   Location: ~/.jarvy/plugins/managers/company-pm.toml
#
# ✓ Package manager 'company-pm' registered
#
# Use in tool definitions:
#   [tool.macos]
#   method = "company-pm"
#   package = "my-tool"

# List available package managers
jarvy managers list

# Output:
# Package Managers
# ================
#
# Built-in:
#   homebrew    macOS package manager
#   apt         Debian/Ubuntu packages
#   dnf         Fedora/RHEL packages
#   pacman      Arch Linux packages
#   winget      Windows Package Manager
#   scoop       Windows command-line installer
#
# Custom:
#   company-pm  Company internal package manager
#
# Detected on this system: homebrew, company-pm
```

**Package manager features:**
- Detect, install, uninstall, upgrade commands
- Version extraction patterns
- Script-based complex logic
- Platform restrictions
- Environment variables

### 6. Local Plugin Development

Easy workflow for creating and testing plugins.

```bash
# Create new plugin project
jarvy plugin new my-plugin

# Output:
# Creating plugin: my-plugin
#
# ✓ Created my-plugin/
#     ├── plugin.toml      # Plugin manifest
#     ├── tools/           # Tool definitions
#     │   └── example.toml
#     ├── hooks/           # Hook scripts
#     │   └── example.toml
#     ├── templates/       # Config templates
#     ├── README.md
#     └── LICENSE
#
# Next steps:
#   cd my-plugin
#   # Edit tool/hook definitions
#   jarvy plugin dev    # Start development mode

# Start development mode
cd my-plugin
jarvy plugin dev

# Output:
# Starting plugin development mode...
#
# Plugin: my-plugin (v0.1.0)
# Watching for changes...
#
# Tools loaded: example-tool
# Hooks loaded: example-hook
#
# The plugin is now active. Test with:
#   jarvy search example-tool
#   jarvy setup --only example-tool
#
# Press Ctrl+C to stop development mode

# Validate plugin before publishing
jarvy plugin validate

# Output:
# Validating plugin: my-plugin
#
# Manifest (plugin.toml):
#   ✓ Name valid
#   ✓ Version valid
#   ✓ Author specified
#   ✓ License specified
#
# Tools:
#   ✓ example-tool - all platforms defined
#
# Hooks:
#   ✓ example-hook - script valid
#
# Templates:
#   ✓ No templates defined
#
# ✓ Plugin is valid and ready for publishing

# Build plugin for distribution
jarvy plugin build

# Output:
# Building plugin: my-plugin
#
# Bundling files...
# Creating archive...
#
# ✓ Built: my-plugin-0.1.0.jarvy-plugin
#   Size: 12.4 KB
#   Tools: 1
#   Hooks: 1
#
# Install locally: jarvy plugin install ./my-plugin-0.1.0.jarvy-plugin
# Publish: jarvy plugin publish ./my-plugin-0.1.0.jarvy-plugin
```

**Plugin development features:**
- Scaffold new plugin projects
- Hot-reload development mode
- Validation before publish
- Build/packaging
- Local testing

## Acceptance Criteria

### Custom Tool Definitions
- [ ] Tool definition TOML format documented
- [ ] `jarvy plugin add-tool` installs definitions
- [ ] `jarvy plugin list-tools` shows custom tools
- [ ] `jarvy plugin remove-tool` removes definitions
- [ ] `jarvy plugin validate-tool` validates format
- [ ] Custom tools work in jarvy.toml
- [ ] Platform-specific install methods work
- [ ] Version detection patterns work

### Plugin Registry
- [ ] `jarvy plugin search` finds plugins
- [ ] `jarvy plugin install` downloads and installs
- [ ] `jarvy plugin list` shows installed plugins
- [ ] `jarvy plugin update` checks for updates
- [ ] `jarvy plugin remove` uninstalls cleanly
- [ ] Integrity verification on install
- [ ] Offline use of installed plugins

### Hook Libraries
- [ ] Hook library TOML format documented
- [ ] `jarvy plugin install-hooks` installs libraries
- [ ] `jarvy plugin list-hooks` shows available hooks
- [ ] `use = "library:hook"` syntax works in jarvy.toml
- [ ] Hooks can specify platform restrictions
- [ ] Hooks can declare tool dependencies
- [ ] Multiple hooks can be composed

### Template Sharing
- [ ] `jarvy templates --community` lists templates
- [ ] `jarvy templates install` downloads templates
- [ ] Installed templates work with `jarvy init`
- [ ] `jarvy templates publish` workflow works
- [ ] Templates can depend on plugins
- [ ] Template versioning supported

### Custom Package Managers
- [ ] Package manager definition format documented
- [ ] `jarvy plugin add-manager` registers managers
- [ ] `jarvy managers list` shows available managers
- [ ] Custom managers work in tool definitions
- [ ] Detection, install, upgrade commands work
- [ ] Environment variable support

### Local Plugin Development
- [ ] `jarvy plugin new` scaffolds project
- [ ] `jarvy plugin dev` enables hot-reload
- [ ] `jarvy plugin validate` checks plugin
- [ ] `jarvy plugin build` creates distributable
- [ ] Development mode loads tools/hooks
- [ ] Clear documentation and examples

## Technical Approach

### Module Structure

```
src/
  plugin/
    mod.rs              # Plugin system core
    loader.rs           # Plugin loading
    registry.rs         # Plugin registry client
    tool_def.rs         # Custom tool definitions
    hook_lib.rs         # Hook libraries
    manager.rs          # Custom package managers
    template.rs         # Template management
    development.rs      # Dev mode
    sandbox.rs          # Security sandbox
  commands/
    plugin.rs           # Plugin CLI commands
```

### Plugin Manifest Format

```toml
# plugin.toml
[plugin]
name = "jarvy-k8s-tools"
version = "1.2.0"
description = "Kubernetes ecosystem tools"
author = "k8s-community"
license = "MIT"
homepage = "https://github.com/k8s-community/jarvy-k8s-tools"
repository = "https://github.com/k8s-community/jarvy-k8s-tools"
keywords = ["kubernetes", "k8s", "devops"]
min_jarvy_version = "0.1.0"

[plugin.tools]
# List of tool definition files
files = ["tools/*.toml"]

[plugin.hooks]
# Hook library files
files = ["hooks/*.toml"]

[plugin.templates]
# Template directories
directories = ["templates/*"]

[plugin.dependencies]
# Other plugins this depends on
jarvy-docker-tools = ">=1.0"
```

### Plugin Loading

```rust
// src/plugin/loader.rs
use std::path::PathBuf;

pub struct PluginLoader {
    plugin_dir: PathBuf,
    loaded: HashMap<String, LoadedPlugin>,
}

impl PluginLoader {
    pub fn load_all(&mut self) -> Result<(), Error> {
        let plugin_dirs = std::fs::read_dir(&self.plugin_dir)?;

        for entry in plugin_dirs {
            let path = entry?.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    let plugin = self.load_plugin(&path)?;
                    self.loaded.insert(plugin.name.clone(), plugin);
                }
            }
        }

        Ok(())
    }

    fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin, Error> {
        let manifest: PluginManifest = toml::from_str(
            &std::fs::read_to_string(path.join("plugin.toml"))?
        )?;

        // Load tool definitions
        let tools = self.load_tool_definitions(path, &manifest)?;

        // Load hook libraries
        let hooks = self.load_hook_libraries(path, &manifest)?;

        Ok(LoadedPlugin {
            name: manifest.plugin.name,
            version: manifest.plugin.version,
            tools,
            hooks,
            path: path.to_owned(),
        })
    }
}
```

### Security Sandbox

```rust
// src/plugin/sandbox.rs
pub struct PluginSandbox {
    allowed_paths: Vec<PathBuf>,
    allowed_env_vars: Vec<String>,
    network_allowed: bool,
}

impl PluginSandbox {
    pub fn new_restricted() -> Self {
        Self {
            allowed_paths: vec![
                dirs::home_dir().unwrap().join(".jarvy"),
            ],
            allowed_env_vars: vec![
                "HOME".to_string(),
                "PATH".to_string(),
                "USER".to_string(),
            ],
            network_allowed: false,
        }
    }

    pub fn validate_script(&self, script: &str) -> Result<(), SecurityError> {
        // Check for dangerous patterns
        let dangerous_patterns = [
            "rm -rf /",
            "sudo ",
            "curl | bash",
            "wget | bash",
        ];

        for pattern in dangerous_patterns {
            if script.contains(pattern) {
                return Err(SecurityError::DangerousPattern(pattern.to_string()));
            }
        }

        Ok(())
    }
}
```

## Implementation Steps

1. Create plugin module structure
2. Define plugin manifest format
3. Implement plugin loader
4. Implement custom tool definitions
5. Build hook library system
6. Implement plugin registry client
7. Add template sharing
8. Create custom package manager support
9. Implement development mode
10. Add security sandbox
11. Build CLI commands
12. Write plugin validation
13. Create documentation
14. Write unit tests
15. Write integration tests

## Dependencies

- `toml` - Manifest parsing (existing)
- `reqwest` - Registry client (for fetching plugins)
- `zip` - Plugin archive handling
- `notify` - File watching for dev mode

## Effort Estimate

| Task | Effort |
|------|--------|
| Plugin module structure | 0.5 days |
| Plugin manifest format | 1 day |
| Plugin loader | 2 days |
| Custom tool definitions | 2 days |
| Hook library system | 2 days |
| Plugin registry client | 2 days |
| Template sharing | 1.5 days |
| Custom package managers | 2 days |
| Development mode | 2 days |
| Security sandbox | 1.5 days |
| CLI commands | 1.5 days |
| Validation | 1 day |
| Testing | 3 days |
| Documentation | 1.5 days |
| **Total** | **23.5 days** |

## Files to Create/Modify

### New Files
- `src/plugin/mod.rs`
- `src/plugin/loader.rs`
- `src/plugin/registry.rs`
- `src/plugin/tool_def.rs`
- `src/plugin/hook_lib.rs`
- `src/plugin/manager.rs`
- `src/plugin/template.rs`
- `src/plugin/development.rs`
- `src/plugin/sandbox.rs`
- `src/commands/plugin.rs`
- `tests/plugin_integration.rs`

### Modified Files
- `src/main.rs` - Add plugin commands
- `src/commands/mod.rs` - Export plugin
- `src/tools/registry.rs` - Integrate plugin tools
- `Cargo.toml` - Add zip, notify
- `CLAUDE.md` - Document plugin system

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Custom tool support | None | Full |
| Community plugins | None | Registry |
| Reusable hooks | Copy-paste | Libraries |
| Template sharing | None | Community |
| Custom managers | None | Extensible |
| Plugin development | None | Scaffolding |

## Risks

1. **Security**: Third-party code execution risks
   - Mitigation: Sandbox, validation, signed plugins (future)

2. **Compatibility**: Plugin format may need changes
   - Mitigation: Version plugin format, deprecation policy

3. **Quality**: Low-quality plugins hurt ecosystem
   - Mitigation: Validation, ratings, featured plugins

4. **Maintenance**: Plugins may become abandoned
   - Mitigation: Version tracking, deprecation warnings

5. **Complexity**: Plugin system adds complexity
   - Mitigation: Clear docs, simple formats, good errors
