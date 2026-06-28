---
title: "Library registry — Jarvy"
description: "Publish reusable AI hooks, MCP servers, and AI agent skills at any HTTPS URL. One manifest format, one trust model, three consumers."
tags:
  - guides
  - ai-hooks
  - mcp
  - skills
---

# Library registry

Want your team to share an AI hook (`block-deploys-to-prod`), an MCP server (`myorg-tickets`), or an AI agent skill (`myorg-code-review`) across every developer's machine, without each one cloning a repo and copying files?

Publish a **library manifest** at any HTTPS URL, then point every consumer at it from their `jarvy.toml`. The three consumers — `[ai_hooks]`, `[mcp_register]`, `[skills]` — all share one manifest format, one fetch pipeline, one cache layout, and one trust model.

This is PRD-054. The pattern is intentionally identical across the three consumers so a publisher writes one manifest and serves it to all three.

---

## Quickstart

### Publisher: write a `manifest.json`

Host this at any HTTPS URL — GitHub Pages, your own CDN, S3, internal Artifactory, anywhere.

```json
{
  "schema_version": 1,
  "publisher": "myorg",
  "description": "MyOrg internal AI guardrails + skills",
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
      "bash": "#!/usr/bin/env bash\nset -e\nif jq -er '.command | test(\"kubectl apply.*prod\")' <<<\"$JSON\"; then\n  echo 'blocked: prod deploys require manual approval' >&2\n  exit 2\nfi",
      "powershell": "if ($json.command -match 'kubectl apply.*prod') { Write-Error 'blocked'; exit 2 }",
      "timeout_ms": 5000
    },
    {
      "kind": "mcp_server",
      "name": "myorg-tickets",
      "version": "0.3.0",
      "description": "Read Linear tickets",
      "command": "myorg-mcp-tickets",
      "args": ["serve"],
      "env": { "LINEAR_API_KEY": "${LINEAR_API_KEY}" },
      "supported_agents": ["claude-code", "cursor"]
    },
    {
      "kind": "skill",
      "name": "myorg-code-review",
      "version": "2.1.0",
      "description": "MyOrg-specific code review checklist",
      "skill_md_url": "https://cdn.myorg.com/jarvy/skills/code-review-2.1.0/SKILL.md",
      "skill_md_sha256": "abc123...",
      "supported_agents": ["claude-code", "cursor", "codex"]
    }
  ]
}
```

That's the whole spec. Any HTTPS URL serving JSON in this shape is a library.

### Consumer: point your `jarvy.toml` at the URL

```toml
[ai_hooks]
agents = ["claude-code", "cursor"]

[[ai_hooks.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"

[[ai_hooks.hook]]
use = "no-prod-deploys"           # resolves from library_sources

[mcp_register]
agents = ["claude-code"]
allow_custom_servers = true       # required to enable library servers

[[mcp_register.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"

[[mcp_register.server]]
use = "myorg-tickets"

[skills]
agents = ["claude-code", "cursor"]

[[skills.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"

[skills.install]
myorg-code-review = "2.1.0"
```

Run `jarvy setup` (or any per-consumer apply command), and Jarvy fetches the manifest, sha-verifies any off-manifest content, and applies the items. Re-running is idempotent.

---

## Manifest format

| Field | Required | Description |
|-------|----------|-------------|
| `schema_version` | yes | `1` today. Bumped only on breaking changes. |
| `publisher` | yes | Short identifier. Used in cache path + telemetry. |
| `description` | no | Human-readable. Surfaced by `jarvy library show`. |
| `homepage` | no | URL for "where to file bugs". Informational. |
| `generated_at` | no | ISO-8601. Informational. |
| `items` | yes | Array of typed items (see below). |

Each item carries a `kind` discriminator. Today: `ai_hook`, `mcp_server`, `skill`.

### `ai_hook` item

```json
{
  "kind": "ai_hook",
  "name": "no-prod-deploys",
  "version": "1.0.0",
  "description": "Block prod deploys",
  "event": "pre_tool_use",
  "matcher": "Bash",
  "bash": "...inline script body...",
  "powershell": "...optional Windows variant...",
  "timeout_ms": 5000
}
```

Either `bash` (inline body) or `bash_url` + `bash_sha256` (off-manifest body that's sha-verified at fetch time). v1 supports the inline form only — `bash_url` parses but fetching it is a follow-up phase.

`event` is one of: `pre_tool_use`, `post_tool_use`, `user_prompt_submit`, `session_start`, `stop`, `pre_compact`, `pre_shell_execution`.

### `mcp_server` item

```json
{
  "kind": "mcp_server",
  "name": "myorg-tickets",
  "version": "0.3.0",
  "description": "...",
  "command": "myorg-mcp-tickets",
  "args": ["serve"],
  "env": { "LINEAR_API_KEY": "${LINEAR_API_KEY}" },
  "supported_agents": ["claude-code", "cursor"]
}
```

`supported_agents` is informational — Jarvy registers with whatever agents the consumer's `agents = [...]` list says, regardless. The field is surfaced as a warning when there's a mismatch.

### `skill` item

```json
{
  "kind": "skill",
  "name": "myorg-code-review",
  "version": "2.1.0",
  "description": "...",
  "skill_md_url": "https://cdn.myorg.com/jarvy/skills/code-review-2.1.0/SKILL.md",
  "skill_md_sha256": "abc123...",
  "supported_agents": ["claude-code", "cursor"]
}
```

`skill_md_sha256` is **required** and **enforced**. Jarvy refuses to install when the fetched body's sha256 doesn't match the manifest entry. A publisher MUST cut a new version + manifest entry when content changes; mutating a versioned artifact in place will surface a clear `library.sha_mismatch` event.

---

## Trust model

| Config origin | `library_sources` allowed? |
|---------------|----------------------------|
| Local (your own `jarvy.toml` or `~/.jarvy/config.toml`) | Yes |
| Remote (`jarvy setup --from <url>`) | **No** — refused with `library.remote_refused` event |

Mirrors `[packages] allow_remote` and `[ai_hooks] allow_custom_commands` semantics. A remote-fetched config may NARROW trust (drop a `library_source` you'd otherwise pull) but never BROADEN it (add a `library_source` you haven't approved). There is no override flag — adding one would defeat the entire purpose.

Teams that want to ship `library_sources` to every developer copy them into each developer's local `~/.jarvy/config.toml` instead.

---

## Signature verification

The config schema supports cosign:

```toml
[[ai_hooks.library_sources]]
url = "https://cdn.myorg.com/jarvy/manifest.json"
require_signature = true                              # default
identity_regexp = "^https://github\\.com/myorg/jarvy-library/.+$"
oidc_issuer = "https://token.actions.githubusercontent.com"
```

Signature verification is **scaffolded but not enforced in v1**. The fields parse and round-trip; `require_signature = false` emits a `library.signature_disabled` event today. Enforcement lands in a follow-up phase, gated on the same cosign integration used by `jarvy registry sync`.

**For production use today**, treat `library_sources` like any other dependency you fetch over HTTPS: pin URLs you trust, audit publisher repos, and assume a malicious publisher can ship a malicious hook until cosign enforcement is in. The `library.signature_disabled` warning will surface the risk every fetch.

---

## Cache

Manifests cache to disk at:

```
~/.jarvy/library.d/<sha256-of-url>/manifest.json
```

The URL hash is collision-free; the directory layout is internal and may change. Use `jarvy library list` (when shipped) or read it directly with `jq`.

Refetch happens on every `apply` / `install` call unless the on-disk copy is fresher than `refresh_interval_secs` (default 86400 = 24h). On network failure, the cached copy is served with a `library.fetch.cached_hit reason="fetch_failed"` event so you can tell from logs that you're running stale.

---

## Telemetry

All events route through the existing OTEL pipeline. Stable contract:

| Event | When | Key fields |
|-------|------|-----------|
| `library.sync.started` | fetch begins | `url`, `require_signature` |
| `library.sync.completed` | fetch + parse OK | `url`, `items_synced`, `ai_hook_count`, `mcp_server_count`, `skill_count`, `from_cache`, `signature_verified` |
| `library.fetch.cached_hit` | served from cache | `url`, `reason` |
| `library.cache.write_failed` | disk-write best-effort failure | `url`, `error` |
| `library.signature_disabled` | `require_signature = false` | `url` |
| `library.remote_refused` | trust-gate refusal | `consumer` |
| `skills.installed` | per-skill install | `skill`, `version`, `agent_count`, `skipped_count` |

---

## Bounds + safety

- HTTPS-only. Non-HTTPS URLs refused at the fetch boundary. (Loopback HTTP is allowed only with `JARVY_LIBRARY_ALLOW_INSECURE_FETCH=1`, for integration tests.)
- Manifest cap: 16 MiB. Per-companion-artifact cap: 1 MiB. Larger needs override or split into multiple libraries.
- Userinfo bypass refused: `http://127.0.0.1:80@attacker/x` is parsed as authority + userinfo and rejected.
- Process cache survives the run; disk cache survives across runs. Both are wiped by `jarvy library clean` (when shipped).

---

## Comparison with `jarvy registry sync`

Both fetch HTTPS-hosted JSON, sha-verify content, and cache locally. Differences:

| | Tools registry | Library registry |
|---|----------------|------------------|
| **Configures** | `~/.jarvy/config.toml`'s `[registry]` (single source) | Per-consumer `library_sources = [...]` in `jarvy.toml` (multiple sources) |
| **Trust gate** | Project configs can't subscribe to a registry | Remote configs can't declare library_sources |
| **Cosign** | Enforced today | Scaffolded; enforcement in follow-up |
| **Items** | Tool definitions (TOML) | AI hooks / MCP servers / skills (JSON, tagged) |
| **Apply** | `jarvy registry sync` (explicit) | Implicit on `jarvy setup` / consumer apply |

The two will likely converge on a shared core in a future Jarvy release. For now they're parallel.

---

## What's next

- Cosign signature enforcement (PRD-054 phase 5)
- `jarvy library {sync, list, show, clean}` subcommand (phase 6)
- `bash_url` / `powershell_url` companion fetch for ai_hook items (today only inline `bash:` bodies are honored)
- Companion file fetch for skill items (today only `SKILL.md` lands; templates / helper scripts skip)
- Public reference library (a community-maintained manifest of common hooks)

Track follow-up under `prd/054-library-registry.md`.

---

## Related

- [AI hooks](ai-hooks.md) — built-in `LIBRARY` const + `library_sources` consumer
- [MCP registration](mcp-registration.md) — built-in `jarvy` server + `library_sources` consumer
- [Skills](skills.md) — PRD-049 install pipeline
