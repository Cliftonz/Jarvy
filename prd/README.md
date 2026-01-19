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
