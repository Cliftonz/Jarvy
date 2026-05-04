# Jarvy Product Requirements Documents

This folder contains PRDs for improving Jarvy based on a comprehensive codebase analysis.

## Priority Matrix

### Tier 1: Critical (High Impact, High Urgency)

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 1 | [Parallel Tool Installation](001-parallel-tool-installation.md) | 10x speedup | 3-4 days | Concurrent tool installation with rayon |
| 2 | [Tool Spec Abstraction](002-tool-spec-abstraction.md) | -80% code | 5-6 days | Eliminate 4,200 lines of duplication |
| 4 | [Semver Version Checking](004-semver-version-checking.md) | Correctness | 3-4 days | Fix broken version matching logic |

### Tier 2: High Value (Enables Adoption)

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 3 | [Post-Install Hooks](003-post-install-hooks.md) | +40% adoption | 4-5 days | Shell scripts after tool installs |
| 8 | [Environment Variables](008-environment-variables-support.md) | +30% adoption | 5 days | .env generation, shell rc updates |
| 10 | [CI Detection](010-ci-detection-integration.md) | CI support | 4-5 days | Auto-detect GitHub Actions, GitLab, etc. |
| 34 | [Enhanced Dependency System](034-enhanced-dependency-system.md) | Accuracy | 5-7 days | Strict vs flexible tool dependencies |
| 35 | [Self-Updating](035-self-updating.md) | Freshness | 12 days | Auto-update via same install method, enabled by default |

### Tier 3: Quality & Stability

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 5 | [Error Handling](005-error-handling-improvements.md) | Reliability | 5-6 days | Replace panics, add retry logic |
| 6 | [Testing Infrastructure](006-testing-infrastructure.md) | +40% coverage | 6-7 days | Mocking, CI matrix, coverage |
| 7 | [Documentation](007-documentation-improvements.md) | DX | 3-4 days | Fix gaps, add guides |
| 11 | [Comprehensive Documentation](011-comprehensive-documentation.md) | DX | 10 days | Full docs for all features |

### Tier 4: Advanced Features

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 9 | [Service Management](009-service-management.md) | Full stacks | 7-8 days | Docker containers, databases |
| 12 | [Release Distribution](012-release-distribution.md) | Adoption | 11 days | Package managers, binaries, install scripts |

### Tier 5: Security & Compliance

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 20 | [Security Scanning Infrastructure](020-security-scanning-infrastructure.md) | Supply chain | 8 days | SAST, SBOM, signing, OpenSSF Scorecard |

### Tier 6: Ecosystem & Integrations

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 21 | [MCP Server](021-mcp-server.md) | LLM integration | 10 days | Expose Jarvy as MCP server for Claude, Cursor, etc. |
| 22 | [Remote Telemetry & Monitoring](022-remote-telemetry-monitoring.md) | Observability | 3-5 days | OTEL metrics, traces, unsupported tool reporting |
| 23 | [Docker MCP Catalog](025-docker-mcp-catalog.md) | Distribution | 2-3 days | Containerize and publish to Docker Desktop MCP catalog |

### Tier 7: Environment & Developer Experience

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 39 | [Language Package Dependencies](039-language-package-dependencies.md) | Full env | 10 days | npm, pip, cargo, gem package installation |
| 40 | [IDE Extension Management](040-ide-extension-management.md) | DX | 7.5 days | VS Code and JetBrains extension installation |
| 41 | [Git Configuration Automation](041-git-configuration-automation.md) | DX | 7 days | User identity, signing, aliases, hooks |
| 42 | [Secrets Management Integration](042-secrets-management-integration.md) | Security | 10 days | 1Password, Vault, AWS Secrets Manager |
| 43 | [Configuration Drift Detection](043-configuration-drift-detection.md) | Stability | 9 days | Detect when environment drifts from config |
| 44 | [Tool Auto-Discovery](044-tool-auto-discovery.md) | DX | 8.5 days | Detect required tools from project files |

### Tier 8: Advanced Capabilities

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 45 | [Dependency Graph Visualization](045-dependency-graph-visualization.md) | Visibility | 8 days | ASCII/DOT/SVG/HTML dependency graphs |
| 46 | [Per-Tool Performance Analytics](046-per-tool-performance-analytics.md) | Insights | 8 days | Track installation times and failures |
| 47 | [Multi-Project/Monorepo Support](047-multi-project-monorepo-support.md) | Scale | 8.5 days | Workspace configs with inheritance |
| 48 | [Pre-Commit Hook Installation](048-pre-commit-hook-installation.md) | Quality | 7 days | pre-commit, husky, lefthook integration |

### Tier 9: Polish & Agent Readiness

| # | PRD | Impact | Effort | Description |
|---|-----|--------|--------|-------------|
| 51 | [Universal Structured Output](051-universal-structured-output.md) | Agent DX | 3-4 days | `--format json` on all commands for AI agents and CI |
| 52 | [Progress Indicators](052-progress-indicators.md) | User DX | 3-4 days | Spinners and progress bars via `indicatif` |

## Recommended Implementation Order

```
Phase 1: Foundation (2-3 weeks)
├── PRD-001: Parallel Installation     ← Biggest user-visible win
├── PRD-004: Semver Checking           ← Fix correctness bug
└── PRD-005: Error Handling            ← Stability improvement

Phase 2: Developer Experience (2-3 weeks)
├── PRD-002: Tool Spec Abstraction     ← Pay down tech debt
├── PRD-007: Documentation             ← Enable contributions
└── PRD-006: Testing Infrastructure    ← Quality gate

Phase 3: Feature Expansion (3-4 weeks)
├── PRD-003: Post-Install Hooks        ← Major feature
├── PRD-008: Environment Variables     ← Major feature
└── PRD-010: CI Detection              ← Enterprise readiness

Phase 4: Advanced (4+ weeks)
└── PRD-009: Service Management        ← Full environment provisioning

Phase 5: Distribution (2+ weeks)
├── PRD-011: Comprehensive Docs        ← User and developer guides
├── PRD-012: Release Distribution      ← Package managers, binaries
└── PRD-035: Self-Updating             ← Auto-update enabled by default

Phase 6: Security & Compliance (1+ week)
└── PRD-020: Security Scanning         ← SAST, SBOM, signing, Scorecard

Phase 7: Ecosystem & Integrations (2+ weeks)
├── PRD-021: MCP Server                ← LLM integration via Claude, Cursor, etc.
└── PRD-022: Remote Telemetry          ← OTEL observability, unsupported tool feedback

Phase 8: Environment & Developer Experience (6+ weeks)
├── PRD-039: Language Package Deps     ← npm, pip, cargo, gem packages
├── PRD-040: IDE Extensions            ← VS Code and JetBrains extensions
├── PRD-041: Git Configuration         ← Identity, signing, aliases
├── PRD-042: Secrets Management        ← 1Password, Vault, AWS SM integration
├── PRD-043: Drift Detection           ← Environment state monitoring
└── PRD-044: Tool Auto-Discovery       ← Project analysis for tool suggestions

Phase 9: Advanced Capabilities (4+ weeks)
├── PRD-045: Dependency Graph          ← Visualization of tool dependencies
├── PRD-046: Performance Analytics     ← Installation timing and insights
├── PRD-047: Monorepo Support          ← Workspace configs with inheritance
└── PRD-048: Pre-Commit Hooks          ← pre-commit, husky, lefthook
```

## Quick Reference

### What Each PRD Solves

| Problem | PRD |
|---------|-----|
| Setup takes too long | [001](001-parallel-tool-installation.md) |
| Adding tools requires too much code | [002](002-tool-spec-abstraction.md) |
| Can't run post-install scripts | [003](003-post-install-hooks.md) |
| Version matching is broken | [004](004-semver-version-checking.md) |
| Panics crash the CLI | [005](005-error-handling-improvements.md) |
| Tests don't catch bugs | [006](006-testing-infrastructure.md) |
| Documentation is incomplete | [007](007-documentation-improvements.md), [011](011-comprehensive-documentation.md) |
| Can't set environment variables | [008](008-environment-variables-support.md) |
| Can't start databases/services | [009](009-service-management.md) |
| Doesn't work in CI/CD | [010](010-ci-detection-integration.md) |
| Must build from source | [012](012-release-distribution.md) |
| Only basic dependency scanning | [020](020-security-scanning-infrastructure.md) |
| LLMs can't install tools safely | [021](021-mcp-server.md) |
| No visibility into unsupported tool requests | [022](022-remote-telemetry-monitoring.md) |
| Tool dependencies are too rigid | [034](034-enhanced-dependency-system.md) |
| Users run outdated versions | [035](035-self-updating.md) |
| Can't install npm/pip/cargo packages | [039](039-language-package-dependencies.md) |
| Must manually install VS Code extensions | [040](040-ide-extension-management.md) |
| Must manually configure Git identity | [041](041-git-configuration-automation.md) |
| Secrets must be configured manually | [042](042-secrets-management-integration.md) |
| No visibility into environment drift | [043](043-configuration-drift-detection.md) |
| Must manually identify required tools | [044](044-tool-auto-discovery.md) |
| Can't see tool dependencies | [045](045-dependency-graph-visualization.md) |
| No insight into slow installations | [046](046-per-tool-performance-analytics.md) |
| Monorepos need multiple configs | [047](047-multi-project-monorepo-support.md) |
| Pre-commit hooks need manual setup | [048](048-pre-commit-hook-installation.md) |
| Some commands lack JSON output | [051](051-universal-structured-output.md) |
| No progress feedback during installs | [052](052-progress-indicators.md) |

### Dependencies Between PRDs

```
PRD-001 (Parallel) ─────────────────────────────┐
                                                ↓
PRD-005 (Errors) ──────────────────────────→ PRD-006 (Testing)
                                                ↓
PRD-002 (Tool Spec) ───────────────────────→ All tool PRDs benefit

PRD-003 (Hooks) ←──────────────────────────── PRD-008 (Env Vars)
       ↓
PRD-009 (Services) depends on hooks architecture

PRD-010 (CI) ←─────────────────────────────── PRD-005 (Errors)

PRD-012 (Distribution) ←────────────────────── PRD-011 (Docs) for README
       ↓
Enables wide adoption via brew/apt/winget/cargo

PRD-020 (Security) ←───────────────────────── PRD-012 (Distribution) for release signing
       ↓
Enables supply chain security, SBOM, OpenSSF Scorecard

PRD-021 (MCP Server) ←─────────────────────── PRD-012 (Distribution) for npm/Docker publishing
       ↓
Enables LLMs to safely install tools via Claude, Cursor, etc.

PRD-022 (Telemetry) ←──────────────────────── PRD-021 (MCP Server) for unsupported tool feedback
       ↓
Enables operational monitoring, OTEL metrics/traces, tool request tracking

PRD-034 (Dependencies) ←─────────────────── PRD-001 (Parallel) for dependency ordering
       ↓
Enables flexible dependencies (kubectl needs any K8s cluster, not all of them)

PRD-035 (Self-Update) ←────────────────────── PRD-012 (Distribution) for release binaries
       ↓
Keeps users on latest version automatically, enabled by default

PRD-039 (Package Deps) ←─────────────────── PRD-003 (Hooks) for post-install scripts
       ↓
Enables npm install, pip install, cargo build after tool installation

PRD-040 (IDE Extensions) ←───────────────── PRD-001 (Parallel) for concurrent extension installs
       ↓
VS Code and JetBrains extensions as part of environment setup

PRD-041 (Git Config) ←───────────────────── PRD-003 (Hooks) for identity prompts
       ↓
Git identity, signing, and aliases automatically configured

PRD-042 (Secrets) ←──────────────────────── PRD-008 (Env Vars) for environment injection
       ↓
Secrets from 1Password, Vault, AWS injected into environment

PRD-043 (Drift) ←────────────────────────── PRD-001 (Parallel) for fast version checks
       ↓
Detects when installed tools drift from expected configuration

PRD-044 (Discovery) ←────────────────────── PRD-002 (Tool Spec) for tool registry
       ↓
Analyzes project files to suggest tools for jarvy.toml

PRD-045 (Graph) ←────────────────────────── PRD-034 (Dependencies) for dependency data
       ↓
Visualizes tool dependencies in ASCII, DOT, SVG, HTML formats

PRD-046 (Analytics) ←────────────────────── PRD-001 (Parallel) for timing data
       ↓
Tracks per-tool installation performance over time

PRD-047 (Monorepo) ←─────────────────────── PRD-003 (Hooks) for per-project hooks
       ↓
Workspace configs with tool inheritance across projects

PRD-048 (Pre-commit) ←───────────────────── PRD-041 (Git Config) for git hook paths
       ↓
Installs pre-commit, husky, or lefthook hooks automatically
```

## Analysis Sources

These PRDs were generated from 6 parallel analysis agents examining:

1. **Architecture & Code Quality** - Code duplication, patterns, structure
2. **Error Handling** - Panics, error types, recovery
3. **Testing Coverage** - Gaps, mocking, CI
4. **Documentation** - Typos, missing guides, DX
5. **Performance** - Parallelization, caching, startup time
6. **Feature Gaps** - Competitive analysis, missing capabilities

## Contributing

When implementing a PRD:

1. Create a branch: `feat/prd-XXX-short-name`
2. Follow the implementation steps in the PRD
3. Update tests per PRD-006 patterns
4. Update docs per PRD-007 patterns
5. Submit PR referencing the PRD number
