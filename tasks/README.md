# PRD-002 Implementation Tasks

This directory contains detailed task specifications for completing the Tool Specification Abstraction (PRD-002).

## Task Overview

| Task | Title | Status | Dependencies | Complexity |
|------|-------|--------|--------------|------------|
| T7 | Auto-Registration | pending | T6 | medium |
| T8 | Migrate Remaining Tools | pending | T6, T5 | high |
| T9 | Cleanup Old Implementations | pending | T8 | low |
| T10 | Update Scaffolding | pending | T8 | low |
| T11 | Documentation | pending | T10 | low |

## Completed Tasks (Phase 1)

- **T1-T6**: ToolSpec structs, methods, macro, and 5 proof-of-concept tool migrations
- See `progress.txt` in project root for details

## Task Dependency Graph

```
T5 (custom_install) ─┐
                     ├──► T8 (migrate 37 tools) ──► T9 (cleanup) ──► T10 (scaffolding) ──► T11 (docs)
T6 (POC migration) ──┤
                     │
T6 ──────────────────┴──► T7 (auto-registration)
```

## Recommended Execution Order

1. **T7** - Auto-registration (can run in parallel with T8)
2. **T8** - Migrate remaining 37 tools (batched for safety)
3. **T9** - Clean up old boilerplate
4. **T10** - Update new-tool scaffolding
5. **T11** - Write documentation

## Task File Format

Each task file contains:
- `id`: Task identifier (T7, T8, etc.)
- `parent_prd`: Parent PRD reference
- `title`: Short description
- `description`: Full description
- `status`: pending | in_progress | completed
- `dependencies`: List of prerequisite tasks
- `subtasks`: Detailed breakdown of work
- `acceptance_criteria`: Definition of done
- `risks`: Known risks and mitigations

## Running Tasks

Tasks can be executed using the ralph-loop pattern:

```bash
# Example: Execute T8 with progress logging
claude --ralph-loop "implement tasks/t8-migrate-remaining-tools.json. Log progress to progress.txt"
```

## Files

- `t7-auto-registration.json` - Implement inventory/linkme for auto-registration
- `t8-migrate-remaining-tools.json` - Migrate 37 remaining tools to ToolSpec
- `t9-cleanup-old-implementations.json` - Remove deprecated boilerplate
- `t10-update-scaffolding.json` - Update cargo-jarvy new-tool command
- `t11-documentation.json` - Write contributor documentation
