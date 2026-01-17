# PRD-032: Support & Community Infrastructure

## Overview

This PRD defines the support and community infrastructure for Jarvy, including GitHub issue templates, FAQ knowledge base, and contributor recognition programs.

## Problem Statement

Users have questions beyond what documentation covers. Without proper community infrastructure, users struggle to get help, contributors lack recognition, and common problems are solved repeatedly instead of documented once.

## Evidence

- No standardized issue templates lead to incomplete bug reports
- Common questions are answered repeatedly without centralized FAQ
- Contributors have no visibility or recognition for their work

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | Create comprehensive GitHub issue templates | P0 |
| FR-2 | Build searchable FAQ knowledge base | P0 |
| FR-3 | Create contributor recognition program | P1 |
| FR-4 | Implement community metrics tracking (GitHub-based) | P2 |
| FR-5 | Create mentorship matching for new contributors | P2 |
| FR-6 | Build community knowledge contribution workflow | P2 |

### Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1 | FAQ search response time | < 200ms |
| NFR-2 | Issue template completion rate | > 80% of required fields |
| NFR-3 | Knowledge base uptime | 99.9% |

## Non-Goals

- Community platform (Discord/Slack) - separate PRD
- Website or web-based FAQ portal - separate PRD
- Interactive troubleshooting wizard
- Automated diagnostic collection/upload service
- Community moderation tools
- Paid support tiers or SLAs
- 24/7 staffed support channels
- Automated AI-based issue resolution
- Integration with enterprise ticketing systems (Zendesk, ServiceNow)
- Phone or video support

## Feature Specification

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
      description: Please run `jarvy doctor` and paste the output
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
# Become a contributor: https://github.com/jarvy-dev/jarvy/blob/main/CONTRIBUTING.md
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
Awarded for helping 50+ community members in GitHub Discussions.

## Monthly Recognition

Each month, we highlight in the README:
- **Contributor of the Month**: Most impactful contributions
- **Rising Star**: Best new contributor

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
```

### GitHub-Based Community Metrics

```bash
# Community health metrics (maintainer tool)
jarvy community stats

# Output:
# Jarvy Community Metrics (from GitHub)
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
# GitHub Discussions (Last 30 days)
# ────────────────────────────────────────────────────────────────────────
#   Questions asked:       34
#   Questions answered:    31  (91% answer rate)
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
```

## Acceptance Criteria

### AC-1: Issue Templates
- [ ] Bug report template captures diagnostic info via `jarvy doctor` output
- [ ] Feature request template is structured and complete
- [ ] Tool request template exists with clear requirements
- [ ] Templates guide users to provide necessary information

### AC-2: FAQ Knowledge Base
- [ ] `jarvy faq` command provides searchable FAQ
- [ ] Common issues have documented solutions
- [ ] FAQ is updated based on support trends
- [ ] Embedded FAQ data compiled into binary

### AC-3: Contributor Recognition
- [ ] CONTRIBUTORS.md is automatically maintained via GitHub Actions
- [ ] CLI shows contributor credits for tools (`jarvy info <tool>`)
- [ ] Badge system tracks contribution milestones
- [ ] Monthly recognition is published in releases

### AC-4: Community Metrics
- [ ] Maintainers can view GitHub-based community health metrics
- [ ] Issue response times are tracked
- [ ] Metrics inform prioritization decisions

## Technical Approach

### Module Structure

```
src/
├── community/
│   ├── mod.rs              # Module exports
│   ├── faq.rs              # FAQ search and display
│   └── contributors.rs     # Contributor display
├── commands/
│   ├── faq.rs              # `jarvy faq` command
│   └── contributors.rs     # `jarvy contributors` command

.github/
├── ISSUE_TEMPLATE/
│   ├── bug_report.yml
│   ├── feature_request.yml
│   ├── tool_request.yml
│   └── config.yml
├── DISCUSSION_TEMPLATE/
│   └── q-and-a.yml
├── workflows/
│   └── contributors.yml    # Auto-update CONTRIBUTORS.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
└── SUPPORT.md

docs/
└── faq/
    ├── index.md
    ├── installation.md
    ├── configuration.md
    ├── troubleshooting.md
    └── ci-cd.md

data/
└── faq.json                # Embedded FAQ database
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
        // Load embedded FAQ database
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

### Contributors Display

```rust
// src/community/contributors.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contributor {
    pub username: String,
    pub name: Option<String>,
    pub role: ContributorRole,
    pub prs_merged: u32,
    pub issues_opened: u32,
    pub reviews: u32,
    pub tools_added: Vec<String>,
    pub badges: Vec<String>,
    pub joined: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContributorRole {
    CoreTeam,
    Contributor,
    FirstTimeContributor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorsDatabase {
    pub core_team: Vec<Contributor>,
    pub contributors: Vec<Contributor>,
    pub last_updated: String,
}

impl ContributorsDatabase {
    pub fn load() -> Result<Self, ContributorsError> {
        let data = include_str!("../../data/contributors.json");
        serde_json::from_str(data).map_err(ContributorsError::Parse)
    }

    pub fn top_contributors(&self, limit: usize) -> Vec<&Contributor> {
        let mut sorted: Vec<_> = self.contributors.iter().collect();
        sorted.sort_by(|a, b| b.prs_merged.cmp(&a.prs_merged));
        sorted.into_iter().take(limit).collect()
    }

    pub fn recent_contributors(&self, days: u32) -> Vec<&Contributor> {
        // Filter by join date within last N days
        self.contributors.iter()
            .filter(|c| Self::within_days(&c.joined, days))
            .collect()
    }

    fn within_days(date_str: &str, days: u32) -> bool {
        // Parse date and check if within range
        true // Simplified
    }
}
```

## Implementation Steps

### Phase 1: Foundation (Week 1-2)
1. Create GitHub issue templates
2. Set up CONTRIBUTING.md and CODE_OF_CONDUCT.md
3. Create SUPPORT.md with guidance
4. Set up GitHub Discussions

### Phase 2: FAQ System (Week 3-4)
5. Create FAQ content structure in docs/faq/
6. Build faq.json database format
7. Implement `jarvy faq` command
8. Add FAQ search functionality
9. Integrate FAQ suggestions on errors

### Phase 3: Recognition (Week 5-6)
10. Create CONTRIBUTORS.md template
11. Build GitHub Action to auto-update contributors
12. Implement `jarvy contributors` command
13. Add contributor credits to `jarvy info <tool>`
14. Define badge criteria and tracking

### Phase 4: Metrics & Polish (Week 7)
15. Implement `jarvy community stats` (GitHub API)
16. Write documentation
17. Test and polish

## Dependencies

- **Internal**: Config system, output formatting
- **PRD-016**: Commands infrastructure (doctor for diagnostics)
- **External**: GitHub Actions for automation

## Effort Estimate

| Phase | Tasks | Days |
|-------|-------|------|
| Design | FAQ content structure | 1 |
| Issue Templates | Bug, feature, tool templates | 2 |
| GitHub Setup | Discussions, CONTRIBUTING, SUPPORT | 1 |
| FAQ System | Database, search, CLI command | 4 |
| Recognition | Contributors tracking, badges | 3 |
| Metrics | GitHub-based stats command | 2 |
| Documentation | Guides, policies | 2 |
| Testing | End-to-end testing | 1 |
| **Total** | | **16 days** |

## Files to Create/Modify

### New Files
- `src/community/mod.rs`
- `src/community/faq.rs`
- `src/community/contributors.rs`
- `src/commands/faq.rs`
- `src/commands/contributors.rs`
- `src/data/faq.json`
- `src/data/contributors.json`
- `.github/ISSUE_TEMPLATE/bug_report.yml`
- `.github/ISSUE_TEMPLATE/feature_request.yml`
- `.github/ISSUE_TEMPLATE/tool_request.yml`
- `.github/ISSUE_TEMPLATE/config.yml`
- `.github/DISCUSSION_TEMPLATE/q-and-a.yml`
- `.github/workflows/contributors.yml`
- `.github/CONTRIBUTING.md`
- `.github/CODE_OF_CONDUCT.md`
- `.github/SUPPORT.md`
- `docs/faq/`
- `CONTRIBUTORS.md`

### Modified Files
- `src/main.rs` - Add new commands
- `src/commands/mod.rs` - Export new commands
- `Cargo.toml` - Add dependencies if needed
- `README.md` - Add community links

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Issue template completion | > 80% | Track required fields filled |
| FAQ search relevance | > 70% satisfaction | User feedback via CLI |
| Community response time | < 24 hours median | GitHub metrics |
| Contributor retention | 40% return contributors | Track repeat contributions |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Low FAQ usage | Medium | Medium | Integrate suggestions on errors |
| FAQ staleness | Medium | Medium | Scheduled review process, version in data |
| Recognition gaming | Low | Low | Manual review for badges |

---

*PRD-032 v2.0 | Support & Community Infrastructure | Priority: Low*
