---
prd: 054-library-registry
title: Library Registry — publish reusable AI hooks, skills, and MCP servers
version: 1.0
status: in_progress
priority: high
estimated_days: 6
created: 2026-06-28
---

# PRD-054: Library Registry

## Overview

Establish a single canonical mechanism — a **library registry** — for
publishing and consuming reusable AI hooks, AI agent skills, and MCP
server definitions from a third-party URL. One manifest format, one fetch
pipeline, one cache layout, one trust model. Three consumers wire into
that single mechanism rather than each reinventing URL fetching, caching,
and signature verification.

Replaces three separate "we'll add library URLs later" stubs in
`[ai_hooks]`, `[mcp_register]`, and `[skills]` (PRD-049) with one
shipped pattern.

## Problem Statement

Today Jarvy ships built-in `[ai_hooks]` library entries (`block-rm-rf`,
etc.) and the `jarvy` MCP server. There is no path for a team or
publisher to ship their own:

- **AI hooks** — only the in-binary `LIBRARY` const + raw inline
  `command = "..."` entries (gated heavily by `allow_custom_commands`).
- **MCP servers** — only the built-in `jarvy` server + raw inline
  `[[mcp_register.server]]` entries (same gating).
- **AI agent skills** — no installation path at all (PRD-049 drafted).

Teams keep solving this with README instructions ("clone these repos
into ~/.claude/skills/") that drift, get out of date, and don't
version-pin.

Meanwhile, the tools registry pattern (`jarvy registry sync`) already
solved the harder problem: HTTPS-bounded fetches, sha256 verification,
cosign signature verification, atomic cache swap, audit telemetry. We
should reuse that scaffolding for the three smaller cases.

## Goals

1. **One manifest format** covers ai_hooks / mcp_servers / skills.
   Adding a fourth kind in the future is a serde tag, not a new module.
2. **Trust model parity with tools registry** — HTTPS-only, bounded
   reads, optional cosign verification with the same defaults.
3. **Idempotent**: re-running `jarvy setup` or any apply command with
   the same `library_sources` produces the same output.
4. **Offline tolerance**: cached manifests are honored when the network
   is down. Fetch failures are advisory, not fatal.
5. **Audit trail**: every fetch + cache hit + verification emits a
   structured telemetry event.

## Non-Goals

- Hosting a public registry. Anyone can publish a `manifest.json` at
  any HTTPS URL — no central server.
- Authoring tooling. Publishers hand-write `manifest.json` or generate
  it with their own build step. Jarvy provides the schema, not the
  publishing UX.
- Skill execution. Skills land on disk; the AI agent reads them.
- Web UI / discovery / search. Out of scope; users find libraries the
  same way they find anything else (GitHub, blog posts, word of mouth).

## Manifest format

A library is a single HTTPS URL that resolves to a JSON document. The
URL can point at either the manifest itself or its parent directory —
when the URL ends in `/`, Jarvy appends `manifest.json`.

```json
{
  "schema_version": 1,
  "publisher": "myorg",
  "description": "Internal MyOrg AI guardrails + skills",
  "homepage": "https://github.com/myorg/jarvy-library",
  "generated_at": "2026-06-28T12:00:00Z",
  "items": [
    {
      "kind": "ai_hook",
      "name": "no-prod-deploys",
      "version": "1.0.0",
      "description": "Block kubectl apply against prod-* contexts",
      "event": "pre_tool_use",
      "matcher": "Bash",
      "bash_url": "https://cdn.myorg.com/jarvy/ai-hooks/no-prod-deploys-1.0.0.sh",
      "bash_sha256": "abc123...",
      "powershell_url": "https://cdn.myorg.com/jarvy/ai-hooks/no-prod-deploys-1.0.0.ps1",
      "powershell_sha256": "def456...",
      "timeout_ms": 5000
    },
    {
      "kind": "mcp_server",
      "name": "myorg-tickets",
      "version": "0.3.0",
      "description": "Reads Linear tickets",
      "command": "myorg-mcp-tickets",
      "args": ["serve"],
      "env": { "LINEAR_API_KEY": "${LINEAR_API_KEY}" }
    },
    {
      "kind": "skill",
      "name": "myorg-code-review",
      "version": "2.1.0",
      "description": "MyOrg-specific code review checklist",
      "skill_md_url": "https://cdn.myorg.com/jarvy/skills/code-review-2.1.0/SKILL.md",
      "skill_md_sha256": "789abc...",
      "supported_agents": ["claude-code", "cursor", "codex"]
    }
  ]
}
```

**`kind` discriminator**: every item carries a `kind` field
(`ai_hook` | `mcp_server` | `skill`) so a single manifest can publish
across all three categories. Consumers filter by their kind during
fetch.

**`version` is mandatory** on every item — pinning is not optional. A
publisher MUST cut a new manifest entry when content changes; mutating
a versioned artifact in place breaks the sha256 verification and
surfaces a clear `library.sha_mismatch` event.

**`*_url` + `*_sha256` pairs** for any artifact that lives off the
manifest (hook scripts, SKILL.md bodies). The manifest itself is the
trust anchor; everything else is sha-verified against it.

## Optional cosign signature

```
manifest.json
manifest.json.sig          # optional cosign signature
manifest.json.cert         # optional cosign cert bundle
```

When present, the consumer pins the expected signing identity in the
library_source config:

```toml
[[ai_hooks.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"
require_signature = true                                  # default true
identity_regexp = "^https://github\\.com/myorg/jarvy-library/.+$"
oidc_issuer = "https://token.actions.githubusercontent.com"
```

`require_signature = false` is the escape hatch for development. It
emits a stderr warning + `library.signature_disabled` event every
fetch, mirroring `[registry] require_signature` semantics.

## Configuration syntax

Each consumer extends its existing block with a `library_sources` array:

```toml
[ai_hooks]
agents = ["claude-code", "cursor"]

[[ai_hooks.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"

[[ai_hooks.hook]]
use = "no-prod-deploys"                  # resolves from library_sources

[mcp_register]
agents = ["claude-code"]
allow_custom_servers = true              # required to enable library_sources

[[mcp_register.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"

[[mcp_register.server]]
use = "myorg-tickets"                    # resolves from library_sources

[skills]
agents = ["claude-code"]

[[skills.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"

[skills.install]
"myorg-code-review" = { version = "2.1.0" }
```

The same URL can serve all three consumers — they filter by `kind`
during ingest.

## Trust model

| Config origin | `library_sources` allowed? |
|---------------|----------------------------|
| `Local` (own `jarvy.toml`) | Yes |
| `Remote` (`jarvy setup --from <url>`) | **No** — refused with `library.remote_refused` event |

Mirrors `[ai_hooks] allow_custom_commands` and
`[packages] allow_remote` semantics. The principle: a remote-fetched
config may NARROW trust (drop a library_source you'd otherwise pull)
but never BROADEN it (add a library_source you haven't approved).

There is no `allow_remote` override for `library_sources` because
adding one would defeat the entire purpose. A team that needs to ship
library URLs in a shared config copies them into each developer's
local `~/.jarvy/config.toml` instead.

## Cache layout

```
~/.jarvy/library.d/
├── <publisher-slug>/
│   ├── <library-hash>/
│   │   ├── manifest.json
│   │   ├── manifest.json.sig          # if signed
│   │   ├── ai_hook/
│   │   │   ├── no-prod-deploys-1.0.0.sh
│   │   │   └── no-prod-deploys-1.0.0.ps1
│   │   └── skill/
│   │       └── myorg-code-review-2.1.0/
│   │           └── SKILL.md
│   └── index.json                     # compiled view across versions
```

`<library-hash>` is a sha256 of the canonical URL — collisions are
not possible. Atomic swap pattern: write to `<library-hash>.new/`,
fsync, then rename. Matches the existing
`crate::registry_remote::cache` flow.

## Telemetry

| Event | When | Fields |
|-------|------|--------|
| `library.sync.started` | fetch + verify begins | `url`, `kind_count`, `require_signature` |
| `library.sync.completed` | success | `url`, `items_synced`, `signature_verified`, `duration_ms` |
| `library.sync.failed` | preflight + per-stage error | `url`, `stage`, `reason` |
| `library.fetch.cached_hit` | served from cache (offline / TTL) | `url`, `age_seconds` |
| `library.sha_mismatch` | per-item sha verify fails | `url`, `item_kind`, `item_name`, `expected`, `actual` |
| `library.signature_disabled` | `require_signature = false` | `url` |
| `library.signature_refused` | cosign rejected | `url`, `identity_regexp`, `oidc_issuer`, `reason` |
| `library.remote_refused` | trust gate refusal | `consumer`, `reason` |
| `library.item_skipped` | `use = "..."` resolution failed | `consumer`, `item_name`, `reason` |

## Shared module

```
src/library_registry/
├── mod.rs              # public API: fetch, verify, list_items, resolve
├── config.rs           # LibrarySource, schema
├── manifest.rs         # Manifest, LibraryItem (tagged by kind)
├── fetch.rs            # HTTPS-only, bounded reads (reuses net::agent)
├── cache.rs            # ~/.jarvy/library.d/ layout
└── signature.rs        # cosign verify (deferred to follow-up if shipped without)
```

Public API:

```rust
pub fn sync(source: &LibrarySource) -> Result<SyncReport, LibraryError>;
pub fn resolve_hook(name: &str) -> Option<LibraryHookItem>;
pub fn resolve_mcp_server(name: &str) -> Option<LibraryMcpItem>;
pub fn resolve_skill(name: &str) -> Option<LibrarySkillItem>;
pub fn list_cached() -> Vec<CachedLibrary>;
```

Consumers reach for `resolve_*` at apply time. The cache is populated
lazily on the first fetch and refreshed on `jarvy setup` per-source
TTL (default 24h, override per source with `refresh_interval`).

## CLI surface

```bash
jarvy library sync               # fetch + verify all sources from jarvy.toml
jarvy library sync --source <url>   # one-off ad hoc fetch
jarvy library list               # show every cached library + item counts
jarvy library show <publisher>   # detail dump of one library
jarvy library clean              # purge cache (forces re-fetch)
```

Plus the existing `jarvy ai-hooks apply` / `jarvy mcp-register apply` /
(new) `jarvy skills install` resolve library entries transparently —
no separate command needed for the consumer side.

## Migration

No migration required. Adding `library_sources` is additive. The
built-in `LIBRARY` const in `src/ai_hooks/library.rs` continues to ship
as the trust anchor for canonical Jarvy-authored hooks — third-party
library entries are looked up AFTER the built-in lookup fails. Name
collision favors the built-in.

## Implementation phases

| Phase | Scope | Effort |
|-------|-------|--------|
| 1 | Shared `library_registry` module + manifest schema + fetch + cache (no sig verify) | 2d |
| 2 | Wire `[ai_hooks] library_sources` + `use = "..."` resolution | 1d |
| 3 | Wire `[mcp_register] library_sources` + `use = "..."` resolution | 0.5d |
| 4 | `src/skills/` module + `jarvy skills` CLI + setup integration | 2d |
| 5 | Cosign signature verification + `library.signature_*` events | 1d |
| 6 | `jarvy library {sync,list,show,clean}` subcommand | 0.5d |
| 7 | Tests + docs | 1d |
| **Total** | | **~8 days** |

v1 ships phases 1–4 (the user-visible surface). Phases 5–7 are
follow-up tracked under this same PRD.

## Success metrics

| Metric | Current | Target |
|--------|---------|--------|
| Teams shipping reusable AI hook libraries | 0 | 5+ public, 50+ private |
| Skills installed via Jarvy | 0 | 80% of project skill installs |
| MCP servers via library_sources vs inline | N/A | 70% library / 30% inline |
| Time from "team wants to share a hook" to "everyone has it" | days (PR + cut release + everyone updates) | minutes (publisher updates manifest, next `jarvy setup` picks it up) |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Hostile library author ships malicious hook | Low | High | Cosign verification + signing-identity pin; documented as required for production use |
| Manifest format churn | Medium | Medium | `schema_version` field; major bumps require explicit opt-in |
| Stale cache hides hotfixes | Medium | Low | Default 24h TTL; `jarvy library sync` forces refresh |
| URL squatting / typo-squatting | Low | High | No central registry to squat on; users pick URLs deliberately. Document the risk in `docs/library-registry.md`. |
| sha256 mismatch on legitimate content change | Low | Low | Publisher must cut new version + manifest entry; documented as required |

## Related

- `prd/049-skills-registry-integration.md` — earlier draft of skills
  installation; PRD-054 supersedes the "where do skills come from"
  half by establishing the shared registry pattern. The skills-specific
  CLI / agent detection / SKILL.md parsing still belongs to PRD-049.
- `prd/021-mcp-server.md` — built-in `jarvy` MCP server. PRD-054 lets
  teams add their own.
- `src/registry_remote/` — tools registry that established the
  HTTPS-bounded fetch + sig verify + atomic cache swap pattern that
  PRD-054 reuses.
