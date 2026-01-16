# PRD-032: Support & Community Infrastructure

## Overview

This PRD defines the support and community infrastructure for Jarvy, including community platforms, issue templates, knowledge base, troubleshooting tools, and contributor recognition programs.

## Problem Statement

Users have questions beyond what documentation covers, and first-party support is expensive and doesn't scale. Without proper community infrastructure, users struggle to get help, contributors lack recognition, and common problems are solved repeatedly instead of documented once.

## Evidence

- Support requests come through disparate channels (GitHub issues, email, social media)
- No standardized issue templates lead to incomplete bug reports
- Common questions are answered repeatedly without centralized FAQ
- Contributors have no visibility or recognition for their work
- Troubleshooting requires manual back-and-forth to gather diagnostics

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | Provide official community platform (Discord/Slack) | P0 |
| FR-2 | Create comprehensive GitHub issue templates | P0 |
| FR-3 | Build searchable FAQ knowledge base | P0 |
| FR-4 | Implement interactive troubleshooting wizard | P1 |
| FR-5 | Create contributor recognition program | P1 |
| FR-6 | Automate diagnostic collection for support | P1 |
| FR-7 | Provide community moderation tools | P2 |
| FR-8 | Implement community metrics dashboard | P2 |
| FR-9 | Create mentorship matching for new contributors | P2 |
| FR-10 | Build community knowledge contribution workflow | P2 |

### Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1 | FAQ search response time | < 200ms |
| NFR-2 | Issue template completion rate | > 80% of required fields |
| NFR-3 | Community response time (median) | < 24 hours |
| NFR-4 | Knowledge base uptime | 99.9% |

## Non-Goals

- Paid support tiers or SLAs
- 24/7 staffed support channels
- Automated AI-based issue resolution
- Integration with enterprise ticketing systems (Zendesk, ServiceNow)
- Phone or video support

## Feature Specification

### Community Platform

```markdown
# Jarvy Community - Discord Server Structure

## Channels

### Information
- #welcome - Server rules and getting started
- #announcements - Release notes, important updates
- #faq - Frequently asked questions (read-only)

### Support
- #help-general - General questions
- #help-installation - Installation issues
- #help-configuration - jarvy.toml and config questions
- #help-ci-cd - CI/CD pipeline integration

### Development
- #contributors - Contributor discussions
- #pull-requests - PR review requests and discussions
- #rfc-discussion - Feature proposals
- #showcase - Share your jarvy.toml configs

### Platforms
- #macos - macOS-specific issues
- #linux - Linux-specific issues
- #windows - Windows-specific issues

## Roles
- @Team - Core maintainers
- @Contributor - Merged PR authors
- @Helper - Community helpers (active in support)
- @Beta Tester - Early release testers

## Bots
- JarvyBot - Automated support assistant
  - Links to relevant docs
  - Suggests issue templates
  - Posts release announcements
  - Tracks community metrics
```

### GitHub Issue Templates

```yaml
# .github/ISSUE_TEMPLATE/bug_report.yml
name: Bug Report
description: Report a bug or unexpected behavior
title: "[Bug]: "
labels: ["bug", "needs-triage"]
body:
  - type: markdown
    attributes:
      value: |
        Thanks for reporting a bug! Please fill out the sections below.

  - type: textarea
    id: description
    attributes:
      label: Bug Description
      description: What happened? What did you expect to happen?
      placeholder: |
        When I run `jarvy setup`, the rust installation fails with...
    validations:
      required: true

  - type: textarea
    id: reproduce
    attributes:
      label: Steps to Reproduce
      description: How can we reproduce this issue?
      placeholder: |
        1. Create jarvy.toml with `rust = "1.75"`
        2. Run `jarvy setup`
        3. See error...
    validations:
      required: true

  - type: textarea
    id: diagnostic
    attributes:
      label: Diagnostic Output
      description: Please run `jarvy diagnose --export` and paste the output
      render: shell
    validations:
      required: true

  - type: dropdown
    id: os
    attributes:
      label: Operating System
      options:
        - macOS (Apple Silicon)
        - macOS (Intel)
        - Linux (Ubuntu/Debian)
        - Linux (Fedora/RHEL)
        - Linux (Arch)
        - Linux (Alpine)
        - Windows 10
        - Windows 11
        - Other
    validations:
      required: true

  - type: input
    id: version
    attributes:
      label: Jarvy Version
      description: Run `jarvy --version` to get this
      placeholder: "jarvy 0.1.0"
    validations:
      required: true

  - type: textarea
    id: config
    attributes:
      label: jarvy.toml (if relevant)
      description: Your configuration file
      render: toml

  - type: checkboxes
    id: checklist
    attributes:
      label: Checklist
      options:
        - label: I searched existing issues and this hasn't been reported
          required: true
        - label: I'm using the latest version of Jarvy
          required: false
```

```yaml
# .github/ISSUE_TEMPLATE/feature_request.yml
name: Feature Request
description: Suggest a new feature or improvement
title: "[Feature]: "
labels: ["enhancement", "needs-triage"]
body:
  - type: markdown
    attributes:
      value: |
        Thanks for suggesting a feature! Please describe what you'd like.

  - type: textarea
    id: problem
    attributes:
      label: Problem Statement
      description: What problem does this solve? Why is this needed?
      placeholder: |
        Currently, when working with multiple projects, I have to...
    validations:
      required: true

  - type: textarea
    id: solution
    attributes:
      label: Proposed Solution
      description: How should this work? Be as specific as possible.
      placeholder: |
        I'd like a command like `jarvy project switch` that...
    validations:
      required: true

  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives Considered
      description: What other solutions have you considered?

  - type: dropdown
    id: scope
    attributes:
      label: Scope
      description: How big is this change?
      options:
        - Small (single command/flag)
        - Medium (new feature, 1-2 weeks work)
        - Large (architectural change)

  - type: checkboxes
    id: contribution
    attributes:
      label: Contribution
      options:
        - label: I'm willing to implement this feature
        - label: I'm willing to test this feature
        - label: I'm willing to write documentation for this feature
```

```yaml
# .github/ISSUE_TEMPLATE/tool_request.yml
name: Tool Request
description: Request support for a new developer tool
title: "[Tool]: "
labels: ["tool-request", "needs-triage"]
body:
  - type: input
    id: tool_name
    attributes:
      label: Tool Name
      placeholder: "kubectl"
    validations:
      required: true

  - type: input
    id: tool_url
    attributes:
      label: Tool Website/Repository
      placeholder: "https://kubernetes.io/docs/tasks/tools/"
    validations:
      required: true

  - type: textarea
    id: use_case
    attributes:
      label: Use Case
      description: Why should Jarvy support this tool? How common is it?
      placeholder: |
        kubectl is essential for Kubernetes development and is used by...
    validations:
      required: true

  - type: textarea
    id: installation
    attributes:
      label: Current Installation Methods
      description: How is this tool currently installed?
      placeholder: |
        - macOS: `brew install kubectl`
        - Linux: `apt install kubectl` or download binary
        - Windows: `winget install Kubernetes.kubectl`

  - type: checkboxes
    id: platforms
    attributes:
      label: Platforms Needed
      options:
        - label: macOS
        - label: Linux
        - label: Windows

  - type: checkboxes
    id: contribution
    attributes:
      label: Contribution
      options:
        - label: I can help implement this tool
        - label: I can help test this tool
```

### FAQ Knowledge Base

```bash
# Built-in FAQ access
jarvy faq

# Output:
# Jarvy FAQ
# ═══════════════════════════════════════════════════════════════════════
#
# Categories:
#   1. Installation (12 articles)
#   2. Configuration (8 articles)
#   3. Troubleshooting (15 articles)
#   4. CI/CD Integration (6 articles)
#   5. Tools (23 articles)
#
# Popular Questions:
#   • How do I install Jarvy on a system without Homebrew?
#   • Why does `jarvy setup` require sudo?
#   • How do I use Jarvy in GitHub Actions?
#   • Can I use different tool versions per project?
#
# Search: jarvy faq search <query>
# Browse: jarvy faq category <name>
# Online: https://jarvy.dev/faq

# Search FAQ
jarvy faq search "github actions"

# Output:
# Search Results for "github actions"
# ═══════════════════════════════════════════════════════════════════════
#
# 1. How do I use Jarvy in GitHub Actions?
#    Category: CI/CD Integration
#    Last updated: 2024-01-10
#
#    Quick answer: Add this to your workflow:
#
#    ```yaml
#    - uses: jarvy-dev/setup-jarvy@v1
#      with:
#        config: jarvy.toml
#    ```
#
#    Full article: jarvy faq show ci-github-actions
#
# 2. How do I cache Jarvy tools in GitHub Actions?
#    Category: CI/CD Integration
#    ...

# Read full article
jarvy faq show ci-github-actions

# Output:
# How do I use Jarvy in GitHub Actions?
# ═══════════════════════════════════════════════════════════════════════
#
# Jarvy provides a GitHub Action for easy CI integration.
#
# ## Basic Setup
#
# ```yaml
# name: CI
# on: [push, pull_request]
# jobs:
#   build:
#     runs-on: ubuntu-latest
#     steps:
#       - uses: actions/checkout@v4
#       - uses: jarvy-dev/setup-jarvy@v1
#         with:
#           config: jarvy.toml
#       - run: your-build-command
# ```
#
# ## With Caching
#
# For faster builds, enable caching:
# ...
#
# ---
# Was this helpful? [y/n]
# Report issue: jarvy faq report ci-github-actions
```

### Troubleshooting Wizard

```bash
# Interactive troubleshooting
jarvy troubleshoot

# Output:
# Jarvy Troubleshooting Wizard
# ═══════════════════════════════════════════════════════════════════════
#
# What issue are you experiencing?
#
#   1. Installation failed for a tool
#   2. Jarvy command not found / won't start
#   3. Configuration file errors
#   4. Permission errors (sudo required)
#   5. Network/download issues
#   6. Tool works but wrong version
#   7. CI/CD pipeline issues
#   8. Other
#
# Select [1-8]: 1

# After selection:
# ═══════════════════════════════════════════════════════════════════════
# Troubleshooting: Installation Failed
# ═══════════════════════════════════════════════════════════════════════
#
# Which tool failed to install?
# > rust
#
# Running diagnostics...
#
# ✓ Checking system compatibility... OK
# ✓ Checking package manager... Homebrew 4.2.0
# ✓ Checking network connectivity... OK
# ✗ Checking rustup installation... FAILED
#
# Issue Identified:
# ────────────────────────────────────────────────────────────────────────
# rustup is already installed but managed outside Jarvy. This can cause
# conflicts.
#
# Recommended Solutions:
#
# 1. Let Jarvy manage Rust (recommended):
#    $ rustup self uninstall
#    $ jarvy setup rust
#
# 2. Skip Rust in Jarvy (use existing):
#    # Add to jarvy.toml:
#    [tools.rust]
#    skip = true
#
# 3. Force reinstall through Jarvy:
#    $ jarvy setup rust --force
#
# Would you like to try solution 1? [y/n]

# Specific tool troubleshooting
jarvy troubleshoot rust

# Network troubleshooting
jarvy troubleshoot --network

# Output:
# Network Diagnostics
# ═══════════════════════════════════════════════════════════════════════
#
# Testing connectivity to common download sources...
#
#   Source                    Status    Latency
#   ───────────────────────────────────────────
#   github.com                ✓ OK      45ms
#   raw.githubusercontent.com ✓ OK      52ms
#   homebrew.bintray.com      ✓ OK      78ms
#   rustup.rs                 ✓ OK      112ms
#   nodejs.org                ✗ SLOW    2340ms
#
# Warning: nodejs.org is responding slowly.
#
# Recommendations:
#   • Use a Node mirror: jarvy config set mirrors.node "https://npmmirror.com"
#   • Check if your network blocks certain domains
#   • Try using a VPN if you're behind a restrictive firewall
```

### Automatic Diagnostic Collection

```bash
# Generate diagnostic bundle for support
jarvy diagnose --export

# Output:
# Generating diagnostic bundle...
#
# Collected:
#   ✓ System information
#   ✓ Jarvy version and configuration
#   ✓ Installed tools and versions
#   ✓ Recent error logs
#   ✓ Network connectivity tests
#   ✓ Package manager status
#
# Sensitive data removed:
#   ✗ API keys and tokens
#   ✗ Private paths sanitized
#   ✗ Environment variables filtered
#
# Bundle created: jarvy-diagnostic-2024-01-15-143052.json
#
# Share options:
#   1. Copy to clipboard: jarvy diagnose --export --clipboard
#   2. Upload to Jarvy (expires in 7 days): jarvy diagnose --export --upload
#   3. Attach to GitHub issue manually

# Upload for easy sharing
jarvy diagnose --export --upload

# Output:
# Uploading diagnostic bundle...
#
# ✓ Uploaded successfully
#
# Share this link in your support request:
#   https://diag.jarvy.dev/d/abc123xyz
#
# This link expires in 7 days and contains:
#   • System: macOS 14.2 (arm64)
#   • Jarvy: 0.1.0
#   • Tools: 12 installed, 2 failed
#   • Errors: 3 recent errors captured
#
# No sensitive information is included.
```

### Contributor Recognition

```bash
# View contributors
jarvy contributors

# Output:
# Jarvy Contributors
# ═══════════════════════════════════════════════════════════════════════
#
# Core Team
# ────────────────────────────────────────────────────────────────────────
#   @maintainer1    Lead maintainer
#   @maintainer2    Core contributor
#
# Top Contributors (All Time)
# ────────────────────────────────────────────────────────────────────────
#   Rank  Contributor      PRs    Issues   Reviews   Tools Added
#   1     @contributor1    45     23       89        12
#   2     @contributor2    34     45       56        8
#   3     @contributor3    28     12       34        15
#   ...
#
# Recent Contributors (Last 30 days)
# ────────────────────────────────────────────────────────────────────────
#   @newcontrib1    Added kubectl tool support
#   @newcontrib2    Fixed Windows installation bug
#   @newcontrib3    Improved documentation
#
# Become a contributor: https://jarvy.dev/contributing
```

### Contributor Recognition Program

```markdown
# Jarvy Contributor Recognition Program

## Contribution Badges

### 🛠️ Tool Master
Awarded for adding 5+ tool definitions to Jarvy.

### 🐛 Bug Hunter
Awarded for reporting 10+ confirmed bugs.

### 📝 Documentation Hero
Awarded for significant documentation contributions.

### 🔧 First PR
Awarded for your first merged pull request.

### 🏆 Core Contributor
Awarded for 25+ merged pull requests.

### 🧪 Quality Champion
Awarded for improving test coverage significantly.

### 💬 Community Helper
Awarded for helping 50+ community members.

## Monthly Recognition

Each month, we highlight:
- **Contributor of the Month**: Most impactful contributions
- **Rising Star**: Best new contributor
- **Helper of the Month**: Most helpful community member

## Recognition in CLI

Contributors are credited in the CLI:
```bash
$ jarvy info kubectl
kubectl - Kubernetes CLI
  Added by: @contributor-name
  Maintained by: @maintainer-name
```

## CONTRIBUTORS.md

All contributors are listed in CONTRIBUTORS.md with:
- GitHub profile link
- Contribution summary
- Badges earned
- Join date

## Swag Program (Future)

Top contributors may receive:
- Jarvy stickers
- T-shirts
- Conference tickets
- Exclusive beta access
```

### Community Metrics Dashboard

```bash
# Community health metrics (maintainer tool)
jarvy community stats

# Output:
# Jarvy Community Metrics
# ═══════════════════════════════════════════════════════════════════════
#
# GitHub (Last 30 days)
# ────────────────────────────────────────────────────────────────────────
#   Issues opened:         45
#   Issues closed:         52  (116% close rate)
#   PRs opened:            28
#   PRs merged:            24  (86% merge rate)
#   Avg time to first response: 4.2 hours
#   Avg time to close:     2.3 days
#
# Discord (Last 30 days)
# ────────────────────────────────────────────────────────────────────────
#   New members:           234
#   Active members:        456
#   Messages:              1,892
#   Questions answered:    167  (89% resolution rate)
#
# Downloads
# ────────────────────────────────────────────────────────────────────────
#   This month:            12,456
#   Total:                 89,234
#   Growth:                +23% MoM
#
# Top Contributors This Month
# ────────────────────────────────────────────────────────────────────────
#   1. @contrib1 - 8 PRs, 12 reviews
#   2. @contrib2 - 5 PRs, 23 comments
#   3. @contrib3 - 4 PRs, 8 issues triaged
```

### Configuration

```toml
# jarvy.toml - Community features
[settings.community]
# Show contributor credits
show_credits = true

# Check for FAQ matches on errors
faq_suggestions = true

# Opt-in to anonymous usage statistics
anonymous_stats = false
```

## Acceptance Criteria

### AC-1: Community Platform
- [ ] Discord server is set up with proper channel structure
- [ ] Moderation tools and bots are configured
- [ ] Welcome flow introduces new members to resources
- [ ] Cross-posting from GitHub releases to announcements

### AC-2: Issue Templates
- [ ] Bug report template captures diagnostic info
- [ ] Feature request template is structured and complete
- [ ] Tool request template exists with clear requirements
- [ ] Templates guide users to provide necessary information

### AC-3: FAQ Knowledge Base
- [ ] `jarvy faq` command provides searchable FAQ
- [ ] Common issues have documented solutions
- [ ] FAQ is updated based on support trends
- [ ] Online version mirrors CLI content

### AC-4: Troubleshooting Wizard
- [ ] Interactive troubleshooting guides users through diagnosis
- [ ] Common issues have automated detection and solutions
- [ ] Diagnostic export sanitizes sensitive data
- [ ] Upload feature creates shareable links

### AC-5: Contributor Recognition
- [ ] CONTRIBUTORS.md is automatically maintained
- [ ] CLI shows contributor credits for tools
- [ ] Badge system tracks contribution milestones
- [ ] Monthly recognition is published

### AC-6: Community Metrics
- [ ] Maintainers can view community health metrics
- [ ] Issue response times are tracked
- [ ] Download and usage trends are visible
- [ ] Metrics inform prioritization decisions

## Technical Approach

### Module Structure

```
src/
├── community/
│   ├── mod.rs              # Module exports
│   ├── faq.rs              # FAQ search and display
│   ├── troubleshoot.rs     # Troubleshooting wizard
│   ├── diagnose.rs         # Diagnostic collection
│   └── contributors.rs     # Contributor display
├── commands/
│   ├── faq.rs              # `jarvy faq` command
│   ├── troubleshoot.rs     # `jarvy troubleshoot` command
│   ├── diagnose.rs         # `jarvy diagnose` command
│   └── contributors.rs     # `jarvy contributors` command

.github/
├── ISSUE_TEMPLATE/
│   ├── bug_report.yml
│   ├── feature_request.yml
│   ├── tool_request.yml
│   └── config.yml
├── DISCUSSION_TEMPLATE/
│   └── q-and-a.yml
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
└── SUPPORT.md

docs/
├── faq/
│   ├── index.md
│   ├── installation.md
│   ├── configuration.md
│   ├── troubleshooting.md
│   └── ci-cd.md
└── community/
    ├── contributing.md
    ├── recognition.md
    └── code-of-conduct.md
```

### FAQ System Implementation

```rust
// src/community/faq.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqEntry {
    pub id: String,
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub content: String,
    pub last_updated: String,
    pub helpful_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqDatabase {
    pub entries: Vec<FaqEntry>,
    pub categories: Vec<String>,
    pub version: String,
}

pub struct FaqSearch {
    db: FaqDatabase,
}

impl FaqSearch {
    pub fn new() -> Result<Self, FaqError> {
        // Load embedded FAQ database or fetch from remote
        let db = Self::load_database()?;
        Ok(Self { db })
    }

    fn load_database() -> Result<FaqDatabase, FaqError> {
        // Embedded FAQ data compiled into binary
        let data = include_str!("../../data/faq.json");
        serde_json::from_str(data).map_err(FaqError::Parse)
    }

    pub fn search(&self, query: &str) -> Vec<&FaqEntry> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<_> = self.db.entries
            .iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&query_lower) ||
                e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
                e.content.to_lowercase().contains(&query_lower)
            })
            .collect();

        // Sort by relevance (title matches first, then tags, then content)
        results.sort_by(|a, b| {
            let a_title = a.title.to_lowercase().contains(&query_lower);
            let b_title = b.title.to_lowercase().contains(&query_lower);
            b_title.cmp(&a_title)
        });

        results
    }

    pub fn get_by_id(&self, id: &str) -> Option<&FaqEntry> {
        self.db.entries.iter().find(|e| e.id == id)
    }

    pub fn get_by_category(&self, category: &str) -> Vec<&FaqEntry> {
        self.db.entries
            .iter()
            .filter(|e| e.category.eq_ignore_ascii_case(category))
            .collect()
    }

    pub fn categories(&self) -> &[String] {
        &self.db.categories
    }
}
```

### Troubleshooting Wizard

```rust
// src/community/troubleshoot.rs

use std::collections::HashMap;

pub struct TroubleshootWizard {
    issues: Vec<IssueCategory>,
}

#[derive(Debug, Clone)]
pub struct IssueCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub diagnostic_steps: Vec<DiagnosticStep>,
    pub solutions: Vec<Solution>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticStep {
    pub name: String,
    pub check: Box<dyn Fn() -> DiagnosticResult + Send + Sync>,
}

#[derive(Debug, Clone)]
pub enum DiagnosticResult {
    Pass,
    Fail(String),
    Warning(String),
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub title: String,
    pub description: String,
    pub commands: Vec<String>,
    pub automatic: bool,
}

impl TroubleshootWizard {
    pub fn new() -> Self {
        Self {
            issues: Self::build_issue_catalog(),
        }
    }

    fn build_issue_catalog() -> Vec<IssueCategory> {
        vec![
            IssueCategory {
                id: "install-failed".into(),
                name: "Installation failed for a tool".into(),
                description: "A tool failed to install during jarvy setup".into(),
                diagnostic_steps: vec![
                    DiagnosticStep {
                        name: "Check system compatibility".into(),
                        check: Box::new(|| {
                            // Check OS, architecture
                            DiagnosticResult::Pass
                        }),
                    },
                    DiagnosticStep {
                        name: "Check package manager".into(),
                        check: Box::new(|| {
                            // Check brew/apt/winget availability
                            DiagnosticResult::Pass
                        }),
                    },
                ],
                solutions: vec![
                    Solution {
                        title: "Reinstall with verbose output".into(),
                        description: "Run setup with debug logging".into(),
                        commands: vec!["jarvy setup --verbose".into()],
                        automatic: false,
                    },
                ],
            },
            // More categories...
        ]
    }

    pub fn run_interactive(&self) -> Result<(), TroubleshootError> {
        // Interactive menu-driven troubleshooting
        todo!()
    }

    pub fn diagnose_tool(&self, tool: &str) -> DiagnosticReport {
        // Run all diagnostics for a specific tool
        todo!()
    }
}
```

### Diagnostic Export

```rust
// src/community/diagnose.rs

use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct DiagnosticBundle {
    pub generated_at: String,
    pub jarvy_version: String,
    pub system: SystemInfo,
    pub config: SanitizedConfig,
    pub tools: Vec<ToolStatus>,
    pub errors: Vec<ErrorLog>,
    pub network: NetworkDiagnostics,
}

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub shell: String,
    pub package_managers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SanitizedConfig {
    pub tools: Vec<String>,
    pub hooks_count: usize,
    pub settings: HashMap<String, String>,
}

impl DiagnosticBundle {
    pub fn collect() -> Result<Self, DiagnosticError> {
        Ok(Self {
            generated_at: chrono::Utc::now().to_rfc3339(),
            jarvy_version: env!("CARGO_PKG_VERSION").to_string(),
            system: SystemInfo::collect()?,
            config: SanitizedConfig::from_config()?,
            tools: ToolStatus::collect_all()?,
            errors: ErrorLog::recent(50)?,
            network: NetworkDiagnostics::run()?,
        })
    }

    pub fn to_json(&self) -> Result<String, DiagnosticError> {
        serde_json::to_string_pretty(self).map_err(DiagnosticError::Serialize)
    }

    pub fn to_clipboard(&self) -> Result<(), DiagnosticError> {
        let json = self.to_json()?;
        // Use clipboard crate
        todo!()
    }

    pub async fn upload(&self) -> Result<String, DiagnosticError> {
        let json = self.to_json()?;
        // Upload to diagnostic service
        // Return URL
        todo!()
    }

    fn sanitize_path(path: &str) -> String {
        // Replace home directory with ~
        // Remove usernames from paths
        // Redact any API keys or tokens
        path.replace(dirs::home_dir().unwrap().to_str().unwrap(), "~")
    }
}
```

## Implementation Steps

### Phase 1: Foundation (Week 1-2)
1. Create GitHub issue templates
2. Set up CONTRIBUTING.md and CODE_OF_CONDUCT.md
3. Create initial FAQ content structure
4. Implement basic `jarvy faq` command

### Phase 2: Community Platform (Week 3-4)
5. Set up Discord server with channel structure
6. Configure moderation bots and automation
7. Create welcome flow and documentation
8. Integrate GitHub notifications

### Phase 3: Troubleshooting (Week 5-6)
9. Implement troubleshooting wizard
10. Create diagnostic collection
11. Build upload service for diagnostic sharing
12. Add error-to-FAQ suggestions

### Phase 4: Recognition & Metrics (Week 7-8)
13. Implement contributor tracking
14. Create CONTRIBUTORS.md automation
15. Add `jarvy contributors` command
16. Build community metrics dashboard
17. Write documentation
18. Launch contributor recognition program

## Dependencies

- **Internal**: Config system, error handling, telemetry
- **PRD-027**: Diagnostic output integration
- **External**: Discord bot framework, GitHub Actions for automation

## Effort Estimate

| Phase | Tasks | Days |
|-------|-------|------|
| Design | Community structure, FAQ content | 2 |
| Issue Templates | Bug, feature, tool templates | 2 |
| FAQ System | Database, search, CLI | 4 |
| Troubleshooting | Wizard, diagnostics, upload | 5 |
| Discord Setup | Channels, bots, automation | 3 |
| Recognition | Contributor tracking, badges | 3 |
| Documentation | Guides, onboarding, policies | 3 |
| Testing | End-to-end, community testing | 2 |
| **Total** | | **24 days** |

## Files to Create/Modify

### New Files
- `src/community/mod.rs`
- `src/community/faq.rs`
- `src/community/troubleshoot.rs`
- `src/community/diagnose.rs`
- `src/community/contributors.rs`
- `src/commands/faq.rs`
- `src/commands/troubleshoot.rs`
- `src/commands/diagnose.rs`
- `src/commands/contributors.rs`
- `src/data/faq.json`
- `.github/ISSUE_TEMPLATE/bug_report.yml`
- `.github/ISSUE_TEMPLATE/feature_request.yml`
- `.github/ISSUE_TEMPLATE/tool_request.yml`
- `.github/CONTRIBUTING.md`
- `.github/CODE_OF_CONDUCT.md`
- `.github/SUPPORT.md`
- `docs/community/`
- `CONTRIBUTORS.md`

### Modified Files
- `src/main.rs` - Add new commands
- `Cargo.toml` - Add dependencies
- `README.md` - Add community links

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Issue template completion | > 80% | Track required fields |
| FAQ search relevance | > 70% satisfaction | User feedback |
| Troubleshoot resolution rate | > 60% | Track wizard completions |
| Community response time | < 24 hours median | Discord/GitHub metrics |
| Contributor retention | 40% return contributors | Track repeat contributions |
| Support volume reduction | 20% fewer basic questions | Compare before/after |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Low community adoption | Medium | Medium | Promote in docs, social |
| Moderation burden | Medium | Low | Bot automation, clear rules |
| FAQ staleness | Medium | Medium | Scheduled review process |
| Diagnostic data privacy | Low | High | Strict sanitization, opt-in upload |
| Recognition gaming | Low | Low | Manual review for badges |

---

*PRD-032 v1.0 | Support & Community Infrastructure | Priority: Low*
