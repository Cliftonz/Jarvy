# PRD-033: Role-Based Configurations

## Overview

Enable teams to define role-based tool requirements that automatically provision different toolsets based on a developer's role (frontend, backend, data science, devops, etc.), reducing configuration complexity while ensuring each team member has the right tools.

## Problem Statement

Development teams have diverse roles with different tool requirements:
- Frontend developers need Node.js, npm, and UI tools
- Backend developers need databases, API tools, and language runtimes
- Data scientists need Python, Jupyter, and ML libraries
- DevOps engineers need Kubernetes, Terraform, and Docker

Currently, teams must either:
1. Create separate config files for each role (fragmentation)
2. Include all tools for everyone (bloated installs)
3. Let developers manually customize (inconsistency)

Role-based configurations solve this by allowing a single team config with role-specific tool sections.

## Evidence

- Teams ask: "How do I give frontend devs different tools than backend?"
- Junior developers don't know which tools they need
- Senior developers want extras that juniors don't require
- Config files become bloated trying to serve all roles
- Manual customization leads to "works on my machine" issues

## Requirements

### Functional Requirements

1. **Role definitions**: Named sections defining tools for each role
2. **Role assignment**: Developers declare their role in local config
3. **Role inheritance**: Roles can extend other roles
4. **Role discovery**: List available roles and their tools
5. **Role comparison**: Diff between roles for understanding

### Non-Functional Requirements

1. Backward compatible with existing jarvy.toml
2. Role resolution adds < 10ms to config parsing
3. Clear error messages for undefined roles
4. Works with config inheritance (PRD-024)

## Non-Goals

- Role enforcement or policies
- Authentication/authorization
- User management or role assignment workflows
- Role-based access control (RBAC) for configs
- Automatic role detection

## Feature Specifications

### 1. Role Definitions

Define roles in team config files.

```toml
# team-config.toml

[tools]
# Base tools for everyone
git = "latest"
jq = "latest"

[roles.frontend]
description = "Frontend development stack"
tools = ["node", "bun", "pnpm"]

[roles.frontend.tools]
node = "20"
bun = "latest"
pnpm = "latest"

[roles.backend]
description = "Backend development stack"
tools = ["go", "docker", "postgresql"]

[roles.backend.tools]
go = "1.22"
docker = "latest"
postgresql = "16"

[roles.data]
description = "Data science and ML stack"
tools = ["python", "jupyter", "duckdb"]

[roles.data.tools]
python = "3.12"
jupyter = "latest"
duckdb = "latest"

[roles.devops]
description = "DevOps and infrastructure stack"
tools = ["terraform", "kubectl", "helm", "docker"]

[roles.devops.tools]
terraform = "latest"
kubectl = "latest"
helm = "latest"
docker = "latest"

[roles.fullstack]
description = "Full-stack development (frontend + backend)"
extends = ["frontend", "backend"]
```

### 2. Role Assignment

Developers declare their role in local jarvy.toml.

```toml
# jarvy.toml (developer's local config)
extends = "https://github.com/company/configs/team-config.toml"
role = "frontend"

[tools]
# Add personal extras beyond role
vim = "latest"
```

**Multiple roles:**

```toml
# jarvy.toml
extends = "team-config.toml"
roles = ["frontend", "devops"]  # Union of both roles
```

**Resolved configuration:**

```bash
jarvy config show --resolved

# Output:
# Resolved Configuration
# ======================
#
# Role: frontend
# Base tools (all roles):
#   git = "latest"
#   jq = "latest"
#
# Role tools (frontend):
#   node = "20"
#   bun = "latest"
#   pnpm = "latest"
#
# Personal additions:
#   vim = "latest"
```

### 3. Role Inheritance

Roles can extend other roles.

```toml
[roles.junior-frontend]
description = "Junior frontend developer"
extends = "frontend"
# Inherits all frontend tools

[roles.senior-frontend]
description = "Senior frontend developer"
extends = "frontend"

[roles.senior-frontend.tools]
# Additional tools for seniors
docker = "latest"
k9s = "latest"

[roles.lead]
description = "Tech lead (full access)"
extends = ["senior-frontend", "senior-backend"]

[roles.lead.tools]
terraform = "latest"
```

**Inheritance rules:**
- Child role inherits all tools from parent
- Child can override parent tool versions
- Multiple inheritance merges all parent tools
- Circular inheritance detection with clear error

### 4. Role Discovery Commands

```bash
# List all available roles
jarvy roles list

# Output:
# Available Roles
# ===============
#
# frontend          Frontend development stack
#   Tools: node, bun, pnpm
#
# backend           Backend development stack
#   Tools: go, docker, postgresql
#
# data              Data science and ML stack
#   Tools: python, jupyter, duckdb
#
# devops            DevOps and infrastructure stack
#   Tools: terraform, kubectl, helm, docker
#
# fullstack         Full-stack development (frontend + backend)
#   Extends: frontend, backend
#   Tools: node, bun, pnpm, go, docker, postgresql

# Show details for a specific role
jarvy roles show frontend

# Output:
# Role: frontend
# ==============
# Description: Frontend development stack
#
# Tools:
#   node = "20"
#   bun = "latest"
#   pnpm = "latest"
#
# Inherited from base:
#   git = "latest"
#   jq = "latest"
#
# Total: 5 tools

# Show role with inheritance chain
jarvy roles show senior-frontend --inheritance

# Output:
# Role: senior-frontend
# =====================
# Description: Senior frontend developer
#
# Inheritance chain:
#   1. frontend (base)
#   2. senior-frontend (final)
#
# Tools by source:
#   From frontend:
#     node = "20"
#     bun = "latest"
#     pnpm = "latest"
#
#   Added in senior-frontend:
#     docker = "latest"
#     k9s = "latest"
```

### 5. Role Comparison

```bash
# Compare two roles
jarvy roles diff frontend backend

# Output:
# Role Comparison: frontend vs backend
# ====================================
#
# Only in frontend:
#   node = "20"
#   bun = "latest"
#   pnpm = "latest"
#
# Only in backend:
#   go = "1.22"
#   postgresql = "16"
#
# In both (same version):
#   docker = "latest"
#
# In both (different version):
#   (none)

# Compare current role to another
jarvy roles diff --current backend

# Output:
# Your Role (frontend) vs backend
# ===============================
#
# You have but backend doesn't:
#   node = "20"
#   bun = "latest"
#   pnpm = "latest"
#
# Backend has but you don't:
#   go = "1.22"
#   postgresql = "16"
```

### 6. Role Override

Override role for a single command.

```bash
# Setup with different role temporarily
jarvy setup --role backend

# Check what would be installed with a role
jarvy setup --role devops --dry-run

# Output:
# Dry Run: Role override to devops
# ================================
#
# Would install (role: devops):
#   terraform = "latest"
#   kubectl = "latest"
#   helm = "latest"
#   docker = "latest"
#
# Your current role (frontend) has:
#   node = "20"
#   bun = "latest"
#   pnpm = "latest"
```

## Acceptance Criteria

### Role Definitions
- [ ] `[roles.name]` section defines a role
- [ ] `description` field documents the role
- [ ] `tools` array lists tool names
- [ ] `[roles.name.tools]` specifies versions
- [ ] Undefined role produces clear error

### Role Assignment
- [ ] `role = "name"` assigns single role
- [ ] `roles = ["a", "b"]` assigns multiple roles
- [ ] Role tools merge with base tools
- [ ] Personal tools add to role tools
- [ ] `jarvy config show --resolved` shows role source

### Role Inheritance
- [ ] `extends = "parent"` inherits tools
- [ ] `extends = ["a", "b"]` multiple inheritance
- [ ] Child overrides parent versions
- [ ] Circular inheritance detected with error
- [ ] Maximum inheritance depth: 5 levels

### Role Commands
- [ ] `jarvy roles list` shows all roles
- [ ] `jarvy roles show <name>` shows role details
- [ ] `jarvy roles show --inheritance` shows chain
- [ ] `jarvy roles diff <a> <b>` compares roles
- [ ] `jarvy roles diff --current <other>` compares to current

### Role Override
- [ ] `--role <name>` overrides for single command
- [ ] `--dry-run` with role shows what would change
- [ ] Override doesn't modify jarvy.toml

## Technical Approach

### Module Structure

```
src/
  roles/
    mod.rs           # Role features
    definition.rs    # Role parsing and validation
    resolver.rs      # Role inheritance resolution
    commands.rs      # CLI commands
```

### Role Definition Types

```rust
// src/roles/definition.rs
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct RoleDefinition {
    pub description: Option<String>,
    pub extends: Option<Extends>,
    pub tools: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Extends {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RoleAssignment {
    Single(String),
    Multiple(Vec<String>),
}
```

### Role Resolution

```rust
// src/roles/resolver.rs
use std::collections::HashSet;

const MAX_DEPTH: usize = 5;

pub struct RoleResolver<'a> {
    roles: &'a HashMap<String, RoleDefinition>,
    visited: HashSet<String>,
    depth: usize,
}

impl<'a> RoleResolver<'a> {
    pub fn resolve(&mut self, role_name: &str) -> Result<ResolvedRole, Error> {
        if self.depth > MAX_DEPTH {
            return Err(Error::MaxDepthExceeded(role_name.to_string()));
        }

        if self.visited.contains(role_name) {
            return Err(Error::CircularInheritance(role_name.to_string()));
        }

        let role = self.roles.get(role_name)
            .ok_or_else(|| Error::UndefinedRole(role_name.to_string()))?;

        self.visited.insert(role_name.to_string());
        self.depth += 1;

        let mut tools = HashMap::new();

        // Resolve parent roles first
        if let Some(extends) = &role.extends {
            let parents = match extends {
                Extends::Single(p) => vec![p.clone()],
                Extends::Multiple(ps) => ps.clone(),
            };

            for parent_name in parents {
                let parent = self.resolve(&parent_name)?;
                tools.extend(parent.tools);
            }
        }

        // Child tools override parent
        tools.extend(role.tools.clone());

        Ok(ResolvedRole {
            name: role_name.to_string(),
            description: role.description.clone(),
            tools,
        })
    }
}

pub struct ResolvedRole {
    pub name: String,
    pub description: Option<String>,
    pub tools: HashMap<String, String>,
}
```

### Config Integration

```rust
// src/config.rs additions

#[derive(Debug, Deserialize)]
pub struct JarvyConfig {
    pub extends: Option<Extends>,
    pub role: Option<RoleAssignment>,
    pub tools: HashMap<String, ToolSpec>,
    pub hooks: Option<HashMap<String, Hook>>,
    pub roles: Option<HashMap<String, RoleDefinition>>,
}

impl JarvyConfig {
    pub fn resolve_with_role(&self) -> Result<ResolvedConfig, Error> {
        let mut tools = self.tools.clone();

        // Add role tools if role specified
        if let Some(role_assignment) = &self.role {
            let role_names = match role_assignment {
                RoleAssignment::Single(r) => vec![r.clone()],
                RoleAssignment::Multiple(rs) => rs.clone(),
            };

            if let Some(roles) = &self.roles {
                let mut resolver = RoleResolver::new(roles);
                for role_name in role_names {
                    let resolved = resolver.resolve(&role_name)?;
                    // Role tools go first, local tools override
                    let mut merged = resolved.tools;
                    merged.extend(tools.clone());
                    tools = merged;
                }
            }
        }

        Ok(ResolvedConfig { tools, ..self.clone() })
    }
}
```

## Implementation Steps

1. Create roles module structure
2. Implement RoleDefinition parsing
3. Implement RoleResolver with inheritance
4. Add role field to JarvyConfig
5. Implement config resolution with roles
6. Implement `jarvy roles list` command
7. Implement `jarvy roles show` command
8. Implement `jarvy roles diff` command
9. Add `--role` flag to setup command
10. Add role information to `config show --resolved`
11. Write unit tests for role resolution
12. Write integration tests
13. Update documentation

## Dependencies

- No new dependencies required
- Uses existing serde, toml parsing

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure | 0.5 days |
| Role definition parsing | 1 day |
| Role resolver | 1.5 days |
| Config integration | 1 day |
| `jarvy roles list` | 0.5 days |
| `jarvy roles show` | 0.5 days |
| `jarvy roles diff` | 1 day |
| `--role` flag | 0.5 days |
| Testing | 1.5 days |
| Documentation | 0.5 days |
| **Total** | **8.5 days** |

## Files to Create/Modify

### New Files
- `src/roles/mod.rs`
- `src/roles/definition.rs`
- `src/roles/resolver.rs`
- `src/roles/commands.rs`
- `tests/roles_integration.rs`

### Modified Files
- `src/main.rs` - Add roles command, --role flag
- `src/config.rs` - Add role field, resolution
- `CLAUDE.md` - Document role features

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Role-based provisioning | None | Supported |
| Tool set customization | Manual | Role-based |
| Team standardization | Per-file | Per-role |

## Risks

1. **Role proliferation**: Teams create too many roles
   - Mitigation: Best practices guide, inheritance

2. **Inheritance complexity**: Deep chains hard to understand
   - Mitigation: Max depth limit, `--inheritance` flag

3. **Role conflicts**: Multiple roles with conflicting versions
   - Mitigation: Last role wins, clear precedence rules

---

*PRD-033 v1.0 | Role-Based Configurations | Priority: Medium*
