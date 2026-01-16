# PRD-024: Team & Enterprise Collaboration

## Overview

Add features that enable teams and enterprises to standardize developer environments across organizations, including shared configuration templates, role-based requirements, configuration inheritance, and SSO integration.

## Problem Statement

Jarvy currently works well for individual developers, but lacks features for team-wide adoption:
- No way to share and enforce team-standard configurations
- No role-based tool requirements (different needs for different roles)
- No configuration inheritance (DRY principle violations)
- No enterprise authentication for private config servers
- Teams manually copy configs, leading to drift and inconsistency

Enterprises need governance over developer tools without creating rigidity that blocks individual productivity.

## Evidence

- Enterprise customers ask: "How do I ensure all devs have the same tools?"
- Teams duplicate configs manually → config drift
- Different roles need different tools (frontend vs backend vs DevOps)
- Security teams want approved tool versions
- Large organizations use SSO for everything

## Requirements

### Functional Requirements

1. **Config inheritance**: `extends` field to compose configurations
2. **Role-based configs**: Tool profiles for different team roles
3. **Team config registry**: Centralized config management
4. **SSO/SAML integration**: Enterprise authentication for private configs
5. **Config validation against team policies**: Ensure compliance
6. **Version pinning enforcement**: Lock down tool versions

### Non-Functional Requirements

1. Works with existing jarvy.toml format (backward compatible)
2. SSO supports common providers (Okta, Azure AD, Google Workspace)
3. Inheritance resolution is fast (< 100ms)
4. Clear error messages for policy violations
5. Graceful degradation when team server unavailable

## Non-Goals

- Multi-tenancy or SaaS platform
- Billing or subscription management
- User management UI (use existing identity providers)
- Real-time config synchronization
- Mobile device management

## Feature Specifications

### 1. Configuration Inheritance

Allow configs to extend other configs.

```toml
# team-base.toml (shared by all team members)
[tools]
git = "latest"
docker = "latest"
jq = "latest"

[hooks.git]
script = "git config --global core.autocrlf input"

---

# jarvy.toml (individual developer)
extends = "https://config.company.com/team-base.toml"

[tools]
# Add role-specific tools
node = "20"
rust = "1.75"

# Override team default
docker = "24.0"  # Need specific version for project

[hooks.custom]
script = "echo 'Project-specific setup'"
```

**Inheritance behavior:**
- Deep merge of tool sections (child overrides parent)
- Hooks are merged (both parent and child run)
- Multiple inheritance: `extends = ["base.toml", "frontend.toml"]`
- Circular dependency detection
- Maximum inheritance depth: 10 levels

```bash
# View resolved configuration
jarvy config show --resolved

# Output:
# Resolved Configuration (3 sources)
# ==================================
#
# Source chain:
#   1. https://config.company.com/team-base.toml
#   2. https://config.company.com/frontend.toml
#   3. ./jarvy.toml (local)
#
# [tools]
# git = "latest"        # from: team-base.toml
# docker = "24.0"       # from: jarvy.toml (overrides team-base)
# jq = "latest"         # from: team-base.toml
# node = "20"           # from: frontend.toml
# rust = "1.75"         # from: jarvy.toml
#
# [hooks]
# git: team-base.toml
# custom: jarvy.toml

# Validate inheritance chain
jarvy validate --check-extends
```

### 2. Role-Based Configurations

Define different tool sets for different roles.

```toml
# team-config.toml
[roles.frontend]
description = "Frontend developers"
tools = ["node", "git", "docker", "jq"]

[roles.backend]
description = "Backend developers"
tools = ["go", "docker", "kubectl", "git", "postgresql"]

[roles.devops]
description = "DevOps engineers"
tools = ["terraform", "kubectl", "helm", "docker", "aws-cli"]

[roles.all]
description = "All team members"
tools = ["git", "docker", "jq"]

# Tool versions (shared across roles)
[tools]
node = "20"
go = "1.21"
docker = "latest"
kubectl = "1.29"
terraform = "1.6"
git = "latest"
jq = "latest"

---

# Individual jarvy.toml
extends = "https://config.company.com/team-config.toml"
role = "frontend"

# Additional personal tools
[tools]
neovim = "latest"
```

```bash
# Setup with specific role
jarvy setup --role frontend

# View available roles
jarvy roles list

# Output:
# Available Roles
# ===============
#
# frontend     Frontend developers (4 tools)
#              node, git, docker, jq
#
# backend      Backend developers (5 tools)
#              go, docker, kubectl, git, postgresql
#
# devops       DevOps engineers (5 tools)
#              terraform, kubectl, helm, docker, aws-cli
#
# all          All team members (3 tools)
#              git, docker, jq

# Show what a role would install
jarvy roles show frontend

# Compare your setup to a role
jarvy roles diff frontend
```

**Role features:**
- Roles defined in team config
- Individual config selects role
- Roles can inherit from other roles
- CLI can override config role
- Role diff shows missing/extra tools

### 3. Team Config Registry

Centralized configuration management for teams.

```bash
# Register a team config source
jarvy team add mycompany https://config.company.com/jarvy/

# Output:
# Added team config source: mycompany
#   URL: https://config.company.com/jarvy/
#   Authentication: None (add with --auth)
#
# Available configs:
#   mycompany/base        - Base tools for all developers
#   mycompany/frontend    - Frontend development stack
#   mycompany/backend     - Backend development stack
#   mycompany/ml          - Machine learning stack

# List registered team sources
jarvy team list

# Output:
# Registered Team Sources
# =======================
#
# mycompany
#   URL: https://config.company.com/jarvy/
#   Auth: Bearer token
#   Configs: 4 available
#   Last sync: 2 hours ago
#
# opensource
#   URL: https://jarvy.dev/community/
#   Auth: None
#   Configs: 25 available
#   Last sync: 1 day ago

# Browse team configs
jarvy team browse mycompany

# Use a team config
jarvy init --from mycompany/frontend

# Sync/update team configs
jarvy team sync

# Remove team source
jarvy team remove mycompany
```

**Registry features:**
- Multiple team sources
- Config discovery (list available)
- Caching with TTL
- Version pinning for configs
- Offline fallback to cached

### 4. SSO/SAML Integration

Enterprise authentication for private config servers.

```bash
# Configure SSO authentication
jarvy auth login --sso

# Output:
# Opening browser for SSO authentication...
# Waiting for authentication... (press Ctrl+C to cancel)
#
# ✓ Authenticated as alice@company.com
# ✓ Token stored securely in system keychain
#
# Organization: Acme Corp
# Roles: developer, frontend-team
# Config access: team-base, frontend, shared-hooks

# Login with specific provider
jarvy auth login --provider okta --domain company.okta.com
jarvy auth login --provider azure --tenant company.onmicrosoft.com
jarvy auth login --provider google --domain company.com

# Check auth status
jarvy auth status

# Output:
# Authentication Status
# =====================
#
# Authenticated: Yes
# User: alice@company.com
# Provider: Okta (company.okta.com)
# Token expires: 2024-01-20 15:30:00
#
# Accessible configs:
#   ✓ https://config.company.com/team-base.toml
#   ✓ https://config.company.com/frontend.toml
#   ✓ https://config.company.com/hooks/shared.toml

# Logout
jarvy auth logout

# Use auth with setup
jarvy setup --from https://config.company.com/frontend.toml
# (automatically uses stored credentials)
```

**SSO features:**
- Browser-based OAuth2/OIDC flow
- Support for Okta, Azure AD, Google Workspace
- Secure token storage (system keychain)
- Token refresh before expiry
- Graceful fallback when auth fails

### 5. Policy Enforcement

Validate configs against team policies.

```toml
# team-policy.toml (managed by team leads/security)
[policy]
name = "Acme Corp Developer Policy"
version = "1.0"

[policy.required_tools]
# These tools must be present
tools = ["git", "docker"]
message = "All developers must have git and docker installed"

[policy.allowed_versions]
# Version constraints for security
docker = ">=24.0"    # CVE fix in 24.0
node = ">=18, <21"   # LTS versions only
python = ">=3.9"     # End of life versions blocked

[policy.blocked_tools]
# Tools not allowed (security reasons)
tools = ["telnet", "ftp"]
message = "Insecure protocols are not allowed"

[policy.required_hooks]
# Hooks that must be configured
tools = ["git"]
message = "Git must have standard hooks configured"
```

```bash
# Validate against team policy
jarvy policy check

# Output:
# Policy Check: Acme Corp Developer Policy v1.0
# =============================================
#
# Required Tools:
#   ✓ git - present
#   ✓ docker - present
#
# Version Compliance:
#   ✓ docker 24.0 - satisfies >=24.0
#   ✗ node 16.0 - requires >=18, <21
#     → Update to node 18 or higher
#
# Blocked Tools:
#   ✓ No blocked tools present
#
# Required Hooks:
#   ✓ git hooks configured
#
# Result: FAILED (1 violation)
#
# Run 'jarvy setup' to fix violations automatically.

# Check policy before setup
jarvy setup --check-policy

# Enforce policy (fail if violations)
jarvy policy enforce --strict

# Show what policy applies
jarvy policy show
```

**Policy features:**
- Required tool checks
- Version range enforcement
- Blocked tool detection
- Required hook validation
- Clear violation messages
- Auto-fix suggestions

### 6. Version Pinning & Lock Files

Lock tool versions for reproducibility.

```bash
# Generate lock file from current state
jarvy lock

# Output:
# Generated jarvy.lock from current environment
#
# Locked versions:
#   git = "2.43.0"
#   node = "20.11.0"
#   docker = "24.0.7"
#   jq = "1.7.1"
#
# This lock file ensures reproducible environments.
# Commit jarvy.lock to version control.

# Setup with locked versions
jarvy setup --locked

# Update lock file
jarvy lock --update
jarvy lock --update node  # Update specific tool

# Show lock status
jarvy lock status

# Output:
# Lock File Status
# ================
#
# git
#   Locked: 2.43.0
#   Installed: 2.43.0
#   Status: ✓ Matches
#
# node
#   Locked: 20.11.0
#   Installed: 20.10.0
#   Status: ✗ Mismatch (installed is older)
#
# docker
#   Locked: 24.0.7
#   Available: 25.0.0
#   Status: ⚠ Update available

# Verify lock integrity
jarvy lock verify
```

**Lock file format:**

```toml
# jarvy.lock
[meta]
generated = "2024-01-15T10:30:00Z"
jarvy_version = "0.1.0"
platform = "darwin-arm64"

[tools.git]
version = "2.43.0"
source = "homebrew"
checksum = "sha256:abc123..."

[tools.node]
version = "20.11.0"
source = "nvm"
checksum = "sha256:def456..."

[tools.docker]
version = "24.0.7"
source = "homebrew-cask"
checksum = "sha256:ghi789..."
```

**Lock features:**
- Exact version pinning
- Platform-specific locks
- Source/method tracking
- Checksum verification
- Selective updates

## Acceptance Criteria

### Configuration Inheritance
- [ ] `extends` field accepts URL or local path
- [ ] Multiple inheritance with array syntax
- [ ] Deep merge of tool sections
- [ ] Hook merging (both parent and child run)
- [ ] Circular dependency detection with clear error
- [ ] Maximum depth enforcement (10 levels)
- [ ] `jarvy config show --resolved` displays merged config
- [ ] Cache for remote configs with TTL

### Role-Based Configurations
- [ ] Roles defined in `[roles.name]` sections
- [ ] `role` field in jarvy.toml selects active role
- [ ] `--role` flag overrides config role
- [ ] `jarvy roles list` shows available roles
- [ ] `jarvy roles show <name>` displays role details
- [ ] `jarvy roles diff <name>` compares to current setup
- [ ] Roles can inherit from base roles

### Team Config Registry
- [ ] `jarvy team add` registers config source
- [ ] `jarvy team list` shows registered sources
- [ ] `jarvy team browse` discovers available configs
- [ ] `jarvy team sync` updates cached configs
- [ ] `jarvy team remove` unregisters source
- [ ] Offline fallback to cached configs
- [ ] Config versioning support

### SSO/SAML Integration
- [ ] `jarvy auth login --sso` opens browser flow
- [ ] Support for Okta, Azure AD, Google Workspace
- [ ] Token stored in system keychain
- [ ] Automatic token refresh
- [ ] `jarvy auth status` shows current auth
- [ ] `jarvy auth logout` clears credentials
- [ ] Graceful degradation when auth unavailable

### Policy Enforcement
- [ ] Policy file format with required/blocked/versions
- [ ] `jarvy policy check` validates current config
- [ ] `jarvy policy show` displays active policy
- [ ] `jarvy policy enforce` fails on violations
- [ ] Clear violation messages with fix suggestions
- [ ] Policy inheritance from team configs
- [ ] `--check-policy` flag on setup

### Version Lock Files
- [ ] `jarvy lock` generates jarvy.lock
- [ ] `jarvy setup --locked` uses lock file
- [ ] `jarvy lock --update` refreshes lock
- [ ] `jarvy lock status` shows drift
- [ ] `jarvy lock verify` checks integrity
- [ ] Platform-specific lock sections
- [ ] Checksum verification for security

## Technical Approach

### Module Structure

```
src/
  team/
    mod.rs            # Team features
    inheritance.rs    # Config inheritance resolution
    roles.rs          # Role-based configuration
    registry.rs       # Team config registry
    policy.rs         # Policy enforcement
  auth/
    mod.rs            # Authentication
    sso.rs            # SSO/OIDC implementation
    keychain.rs       # Secure token storage
    providers/
      okta.rs
      azure.rs
      google.rs
  lock/
    mod.rs            # Lock file management
    generate.rs       # Lock file generation
    verify.rs         # Lock verification
```

### Inheritance Resolution

```rust
// src/team/inheritance.rs
use std::collections::HashSet;

const MAX_DEPTH: usize = 10;

pub struct InheritanceResolver {
    cache: ConfigCache,
    visited: HashSet<String>,
    depth: usize,
}

impl InheritanceResolver {
    pub fn resolve(&mut self, config_path: &str) -> Result<Config, Error> {
        if self.depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded);
        }

        if self.visited.contains(config_path) {
            return Err(Error::CircularDependency(config_path.to_string()));
        }

        self.visited.insert(config_path.to_string());
        self.depth += 1;

        let config = self.load_config(config_path)?;

        if let Some(extends) = &config.extends {
            let parents = match extends {
                Extends::Single(path) => vec![path.clone()],
                Extends::Multiple(paths) => paths.clone(),
            };

            let mut merged = Config::default();
            for parent_path in parents {
                let parent = self.resolve(&parent_path)?;
                merged = merged.merge(parent);
            }
            merged = merged.merge(config);
            Ok(merged)
        } else {
            Ok(config)
        }
    }
}
```

### SSO Implementation

```rust
// src/auth/sso.rs
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge};

pub struct SsoAuth {
    provider: Box<dyn OidcProvider>,
    keychain: Keychain,
}

impl SsoAuth {
    pub async fn login(&self) -> Result<Token, Error> {
        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Build authorization URL
        let (auth_url, csrf_token) = self.provider
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Open browser
        webbrowser::open(auth_url.as_str())?;

        // Start local callback server
        let code = self.wait_for_callback(csrf_token).await?;

        // Exchange code for token
        let token = self.provider
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async()
            .await?;

        // Store in keychain
        self.keychain.store("jarvy_token", &token)?;

        Ok(token)
    }
}
```

## Implementation Steps

1. Create team module structure
2. Implement config inheritance resolver
3. Add remote config fetching with caching
4. Implement role-based configuration
5. Build team config registry
6. Create policy enforcement system
7. Implement SSO authentication flow
8. Add keychain integration for secure storage
9. Implement lock file generation
10. Add lock verification and status
11. Write unit tests for inheritance
12. Write integration tests for SSO
13. Write policy validation tests
14. Update documentation

## Dependencies

- `oauth2` - OAuth2/OIDC client
- `webbrowser` - Open browser for SSO
- `keyring` - Secure credential storage
- `sha2` - Checksum verification

## Effort Estimate

| Task | Effort |
|------|--------|
| Team module structure | 0.5 days |
| Config inheritance | 2 days |
| Remote config caching | 1 day |
| Role-based configuration | 1.5 days |
| Team config registry | 2 days |
| Policy enforcement | 2 days |
| SSO authentication | 3 days |
| Keychain integration | 1 day |
| Lock file generation | 1.5 days |
| Lock verification | 1 day |
| Testing | 3 days |
| Documentation | 1 day |
| **Total** | **19.5 days** |

## Files to Create/Modify

### New Files
- `src/team/mod.rs`
- `src/team/inheritance.rs`
- `src/team/roles.rs`
- `src/team/registry.rs`
- `src/team/policy.rs`
- `src/auth/mod.rs`
- `src/auth/sso.rs`
- `src/auth/keychain.rs`
- `src/auth/providers/okta.rs`
- `src/auth/providers/azure.rs`
- `src/auth/providers/google.rs`
- `src/lock/mod.rs`
- `src/lock/generate.rs`
- `src/lock/verify.rs`
- `tests/inheritance_integration.rs`
- `tests/sso_integration.rs`
- `tests/policy_integration.rs`
- `tests/lock_integration.rs`

### Modified Files
- `src/main.rs` - Add team, auth, lock commands
- `src/config.rs` - Add extends, role fields
- `Cargo.toml` - Add oauth2, keyring dependencies
- `CLAUDE.md` - Document team features

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Config sharing | Manual copy | Inheritance |
| Role management | None | Built-in |
| Team config discovery | None | Registry |
| Enterprise auth | None | SSO |
| Policy enforcement | None | Automated |
| Version reproducibility | None | Lock files |

## Risks

1. **SSO complexity**: OAuth2/OIDC flows vary by provider
   - Mitigation: Start with top 3 providers, abstract differences

2. **Inheritance conflicts**: Deep merge can have unexpected results
   - Mitigation: Clear precedence rules, `--resolved` flag to inspect

3. **Offline team configs**: Network issues block setup
   - Mitigation: Aggressive caching, offline fallback

4. **Policy friction**: Strict policies frustrate developers
   - Mitigation: Clear explanations, auto-fix suggestions

5. **Lock file drift**: Easy to forget to update lock
   - Mitigation: CI integration, warnings on mismatch

6. **Keychain compatibility**: Different per OS
   - Mitigation: Use `keyring` crate with fallback
