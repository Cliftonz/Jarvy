# PRD-023: Docker Desktop MCP Catalog Distribution

## Status: Draft
## Priority: P1
## Effort: Medium (1-2 weeks)
## Depends On: PRD-021 (MCP Server Implementation)

---

## Overview

Containerize Jarvy's MCP server and submit it to Docker Desktop's MCP catalog, enabling one-click installation for developers using Claude Desktop, Cursor, and other MCP clients through Docker's secure distribution infrastructure.

---

## Problem Statement

PRD-021 defines the Jarvy MCP server implementation, but doesn't address distribution via Docker Desktop's MCP catalog. Docker Desktop's MCP Toolkit provides:

1. **One-click installation** - Users discover and enable MCP servers without manual configuration
2. **Automatic updates** - New versions are automatically rebuilt and distributed
3. **Enhanced security** - Docker-built images include cryptographic signatures, provenance tracking, and SBOMs
4. **Enterprise visibility** - IT teams can see/approve MCP servers used by developers
5. **Discoverability** - Listed in Docker Hub's MCP catalog (hub.docker.com/mcp)

To reach the largest audience of developers using Claude Desktop, Cursor, and other MCP clients, Jarvy needs to be in this catalog.

---

## Goals

1. **Containerize** the Jarvy MCP server with secure, minimal Docker images
2. **Submit to Docker MCP Registry** via pull request to `docker/mcp-registry`
3. **Enable secure host tool installation** - Container communicates with host package managers safely
4. **Zero-config experience** - Works out-of-the-box via Docker Desktop MCP Toolkit
5. **Automatic publishing** - New Jarvy releases trigger registry updates

---

## Non-Goals

- Hosting our own Docker images (Docker will build and host as `mcp/jarvy`)
- SSE/HTTP transport (stdio only for Docker MCP catalog)
- Installing tools inside the container (tools install on the host)
- Remote server deployment (local containerized server only)

---

## Target Audience

| User Type | Use Case |
|-----------|----------|
| Individual developers | One-click MCP server setup via Docker Desktop |
| Enterprise teams | Approved MCP server from trusted Docker catalog |
| AI-assisted developers | Claude/Cursor users needing tool installation |
| DevOps engineers | Standardized dev environment provisioning |

---

## Requirements

### Docker MCP Registry Requirements

Per [docker/mcp-registry CONTRIBUTING.md](https://github.com/docker/mcp-registry/blob/main/CONTRIBUTING.md):

| Requirement | Status | Notes |
|-------------|--------|-------|
| MIT or Apache 2.0 license | ✅ | Jarvy is MIT licensed |
| Dockerfile in repo | ❌ | Create `Dockerfile.mcp` |
| `server.yaml` manifest | ❌ | Create MCP registry metadata |
| `tools.json` (tool definitions) | ❌ | Pre-define MCP tools for catalog |
| `readme.md` documentation | ❌ | MCP-specific documentation |
| MCP protocol compliance | ✅ | Implemented in PRD-021 |
| No GPL dependencies | ✅ | Verify before submission |

### Functional Requirements

#### FR-1: Dockerfile for MCP Server

Create a multi-stage Dockerfile that:
- Builds Jarvy with MCP features enabled
- Produces a minimal runtime image (< 20MB)
- Runs as non-root user
- Supports stdio transport for MCP communication

#### FR-2: Host System Integration

The containerized MCP server must:
- Detect host operating system and available package managers
- Execute tool checks via mounted host binaries
- Request tool installations through host package manager communication
- Support macOS, Linux, and Windows hosts

#### FR-3: Security Model

Implement secure container-to-host communication:
- Read-only access to host binary paths for tool detection
- Controlled execution channel for installations
- Audit logging of all operations
- User confirmation for destructive actions

#### FR-4: Registry Metadata

Create Docker MCP Registry manifest with:
- Tool definitions matching PRD-021 MCP tools
- Configuration parameters for enterprise environments
- Clear documentation and usage examples

### Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| Image size | < 20MB (compressed) |
| Startup time | < 200ms to first MCP response |
| Memory usage | < 30MB RSS at idle |
| MCP protocol version | 2024-11-05 or later |
| Supported platforms | linux/amd64, linux/arm64 |

---

## Technical Design

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        MCP Client                                     │
│              (Claude Desktop / Cursor / VS Code)                      │
└─────────────────────────────────────────────────────────────────────┘
                              │ stdio (JSON-RPC)
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Docker Container (mcp/jarvy)                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Jarvy MCP Server                              │ │
│  │  • Handles MCP protocol (tools/list, tools/call, etc.)          │ │
│  │  • Rate limiting, allowlist/denylist                            │ │
│  │  • Audit logging                                                 │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                              │                                        │
│                              ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                  Host Integration Layer                          │ │
│  │  • Detect package managers via mounted paths                    │ │
│  │  • Check tool versions via command execution                    │ │
│  │  • Request installations via Docker socket/API                  │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (volume mounts / Docker Desktop integration)
┌─────────────────────────────────────────────────────────────────────┐
│                         Host System                                   │
│  • /usr/local/bin, /opt/homebrew (macOS)                            │
│  • /usr/bin, /usr/local/bin (Linux)                                 │
│  • Package managers: brew, apt, dnf, winget                          │
└─────────────────────────────────────────────────────────────────────┘
```

### 1. Dockerfile

```dockerfile
# Dockerfile.mcp
# Multi-stage build for Jarvy MCP Server
# Optimized for Docker Desktop MCP Catalog distribution

# ============================================================================
# Stage 1: Build Environment
# ============================================================================
FROM rust:1.83-alpine AS builder

# Install build dependencies for static linking
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf

WORKDIR /build

# Cache dependency compilation
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release

# Build actual application
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin jarvy && \
    cp target/release/jarvy /jarvy

# Verify binary works
RUN /jarvy --version

# ============================================================================
# Stage 2: Runtime Environment
# ============================================================================
FROM alpine:3.21 AS runtime

# Install minimal runtime dependencies
# ca-certificates: for HTTPS (telemetry, future features)
# tini: proper signal handling for containers
RUN apk add --no-cache ca-certificates tini

# Create non-root user for security
RUN addgroup -g 1000 jarvy && \
    adduser -u 1000 -G jarvy -s /bin/sh -D jarvy

# Copy binary from builder
COPY --from=builder /jarvy /usr/local/bin/jarvy

# Create directories for configuration and logs
RUN mkdir -p /home/jarvy/.jarvy && \
    chown -R jarvy:jarvy /home/jarvy

# Switch to non-root user
USER jarvy
WORKDIR /home/jarvy

# Environment defaults
ENV JARVY_MCP_MODE=1
ENV RUST_LOG=warn

# Use tini as init system for proper signal handling
ENTRYPOINT ["/sbin/tini", "--", "jarvy", "mcp"]

# Container metadata (OCI labels)
LABEL org.opencontainers.image.title="Jarvy MCP Server" \
      org.opencontainers.image.description="Safe cross-platform development tool installation for LLMs" \
      org.opencontainers.image.vendor="Jarvy" \
      org.opencontainers.image.source="https://github.com/jarvy-dev/jarvy" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.documentation="https://github.com/jarvy-dev/jarvy/blob/main/docs/mcp/README.md"
```

### 2. server.yaml (MCP Registry Manifest)

```yaml
# mcp/server.yaml
# Docker MCP Registry manifest for Jarvy
# Reference: https://github.com/docker/mcp-registry/blob/main/CONTRIBUTING.md

name: jarvy
type: local

meta:
  category: devops
  tags:
    - devtools
    - installation
    - package-manager
    - cross-platform
    - developer-experience
    - cli

about:
  title: Jarvy - Safe Tool Installation
  description: |
    Install development tools safely across macOS, Linux, and Windows.

    Jarvy provides LLMs with accurate, platform-specific installation
    commands and verification, eliminating hallucinated package names
    and incorrect installation instructions.

    **Key Features:**
    - 100+ supported tools (git, docker, node, python, rust, go, kubectl, etc.)
    - Platform-aware installation (Homebrew, apt, dnf, winget)
    - Safe by default - dry-run mode prevents accidental installations
    - Version checking and verification
    - Rate limiting and audit logging

    **Safety:**
    All installations default to dry-run mode. Actual installations
    require explicit confirmation to prevent unintended changes.
  icon: https://raw.githubusercontent.com/jarvy-dev/jarvy/main/assets/jarvy-icon.png

source:
  project: https://github.com/jarvy-dev/jarvy
  dockerfile: Dockerfile.mcp
  # branch and commit are auto-populated by Docker's build system

config:
  description: |
    Configure Jarvy MCP server behavior. Most users need no configuration.

    **Enterprise Configuration:**
    - Set allowlist to restrict which tools can be installed
    - Set denylist to block specific tools
    - Disable confirmation prompts for automated environments (not recommended)

  # Environment variables for configuration
  env:
    - name: JARVY_MCP_ALLOWLIST
      description: "Comma-separated list of tools to allow (empty = all tools)"
      example: "git,docker,node,python,rust"
      required: false
    - name: JARVY_MCP_DENYLIST
      description: "Comma-separated list of tools to block"
      example: "brew,apt"
      required: false
    - name: JARVY_MCP_REQUIRE_CONFIRMATION
      description: "Require user confirmation for installations"
      example: "true"
      required: false
    - name: JARVY_LOG_LEVEL
      description: "Logging verbosity (error, warn, info, debug)"
      example: "warn"
      required: false

  # Parameters shown in Docker Desktop UI
  parameters:
    type: object
    properties:
      allowlist:
        type: string
        title: "Tool Allowlist"
        description: "Comma-separated list of allowed tools (empty = all)"
      denylist:
        type: string
        title: "Tool Denylist"
        description: "Comma-separated list of blocked tools"
      require_confirmation:
        type: boolean
        default: true
        title: "Require Confirmation"
        description: "Prompt before installing tools"
```

### 3. tools.json (MCP Tool Definitions)

```json
[
  {
    "name": "jarvy_list_tools",
    "description": "List all development tools Jarvy can install. Filter by category (language, database, container, cli, editor, kubernetes) or search by name. Returns tool names, descriptions, and platform support.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "category": {
          "type": "string",
          "enum": ["language", "database", "container", "cli", "editor", "kubernetes", "cloud", "all"],
          "description": "Filter tools by category"
        },
        "platform": {
          "type": "string",
          "enum": ["macos", "linux", "windows", "current"],
          "description": "Filter by platform support (default: current host platform)"
        },
        "search": {
          "type": "string",
          "description": "Search tools by name or description"
        }
      }
    }
  },
  {
    "name": "jarvy_get_tool",
    "description": "Get detailed information about a specific tool including installation methods for all platforms, package manager commands, and any post-install hooks.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "description": "Tool name (e.g., 'git', 'docker', 'node', 'python', 'rust')"
        }
      },
      "required": ["name"]
    }
  },
  {
    "name": "jarvy_check_tool",
    "description": "Check if a specific tool is installed on the system and get its version. Returns installation status, version number, and binary path.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "description": "Tool name to check (e.g., 'git', 'node')"
        }
      },
      "required": ["name"]
    }
  },
  {
    "name": "jarvy_check_multiple",
    "description": "Check installation status of multiple tools at once. Useful for verifying development environment setup or project dependencies.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "tools": {
          "type": "array",
          "items": { "type": "string" },
          "description": "List of tool names to check",
          "examples": [["git", "node", "docker"], ["python", "pip", "virtualenv"]]
        }
      },
      "required": ["tools"]
    }
  },
  {
    "name": "jarvy_install_tool",
    "description": "Install a development tool. IMPORTANT: By default this returns a preview (dry_run=true) showing what command would be executed. Set dry_run=false to actually install, which will prompt for user confirmation.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "description": "Tool name to install (e.g., 'ripgrep', 'jq', 'fzf')"
        },
        "version": {
          "type": "string",
          "description": "Version to install (default: 'latest')"
        },
        "dry_run": {
          "type": "boolean",
          "default": true,
          "description": "If true (default), preview the installation command without executing. Set to false to actually install."
        }
      },
      "required": ["name"]
    }
  },
  {
    "name": "jarvy_platform_info",
    "description": "Get information about the current platform including OS, version, architecture, and available package managers. Useful for understanding what installation methods are available.",
    "inputSchema": {
      "type": "object",
      "properties": {},
      "additionalProperties": false
    }
  }
]
```

### 4. readme.md (MCP Documentation)

```markdown
# Jarvy MCP Server

Safe cross-platform development tool installation for LLMs.

## Overview

Jarvy enables AI assistants (Claude, GPT, etc.) to accurately install development
tools across macOS, Linux, and Windows. Unlike LLMs that may hallucinate package
names or installation commands, Jarvy provides verified, platform-specific
installation instructions.

## Features

- **100+ supported tools**: git, docker, node, python, rust, go, kubectl, terraform, and more
- **Platform-aware**: Uses the correct package manager (Homebrew, apt, dnf, winget)
- **Safe by default**: Dry-run mode prevents accidental installations
- **Version verification**: Check installed versions before and after installation
- **Enterprise-ready**: Allowlist/denylist and audit logging

## Quick Start

### Docker Desktop (Recommended)

1. Open Docker Desktop
2. Navigate to MCP Toolkit
3. Find "Jarvy" in the catalog
4. Click "Enable"
5. Start using with Claude Desktop or Cursor

### Manual Configuration

For Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "jarvy": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "mcp/jarvy"]
    }
  }
}
```

## Available Tools

### jarvy_list_tools
List available tools with optional filtering.

**Example:**
```
List all CLI tools: jarvy_list_tools(category: "cli")
Search for JSON tools: jarvy_list_tools(search: "json")
```

### jarvy_check_tool
Check if a tool is installed.

**Example:**
```
Is git installed? jarvy_check_tool(name: "git")
→ { "installed": true, "version": "2.43.0", "path": "/usr/bin/git" }
```

### jarvy_install_tool
Install a tool (dry-run by default).

**Example:**
```
Preview: jarvy_install_tool(name: "ripgrep")
→ { "dry_run": true, "command": "brew install ripgrep" }

Install: jarvy_install_tool(name: "ripgrep", dry_run: false)
→ [User confirmation prompt]
→ { "success": true, "version": "14.1.0" }
```

## Security

- **Dry-run default**: All installations preview first
- **User confirmation**: Actual installs require explicit approval
- **Rate limiting**: Prevents rapid-fire installations
- **Audit logging**: All actions logged for review
- **Non-root container**: Runs as unprivileged user

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `JARVY_MCP_ALLOWLIST` | Comma-separated allowed tools | (all) |
| `JARVY_MCP_DENYLIST` | Comma-separated blocked tools | (none) |
| `JARVY_MCP_REQUIRE_CONFIRMATION` | Require install confirmation | `true` |

### Enterprise Example

Restrict to approved tools only:

```bash
docker run -i --rm \
  -e JARVY_MCP_ALLOWLIST="git,docker,node,python" \
  -e JARVY_MCP_DENYLIST="brew" \
  mcp/jarvy
```

## Supported Tools

<details>
<summary>Languages & Runtimes</summary>

- node, deno, bun
- python, pyenv
- rust, rustup
- go
- java, gradle, maven
- ruby, rbenv
</details>

<details>
<summary>Containers & Kubernetes</summary>

- docker
- kubectl, helm
- k9s, kind, minikube
</details>

<details>
<summary>CLI Utilities</summary>

- git, gh (GitHub CLI)
- jq, yq
- ripgrep, fd, fzf
- curl, wget, httpie
</details>

<details>
<summary>Databases</summary>

- postgresql
- mysql
- redis
- mongodb
</details>

## Links

- [GitHub Repository](https://github.com/jarvy-dev/jarvy)
- [Full Documentation](https://github.com/jarvy-dev/jarvy/blob/main/docs/mcp/README.md)
- [Report Issues](https://github.com/jarvy-dev/jarvy/issues)
```

### 5. Rust Dependencies for MCP

Add to `Cargo.toml`:

```toml
[dependencies]
# MCP Server Implementation (choose one approach)

# Option A: Use rust-mcp-sdk (recommended)
rust-mcp-sdk = { version = "0.9", default-features = false, features = ["server", "macros", "stdio"] }

# Option B: Manual implementation with lower-level crates
# serde = { version = "1.0", features = ["derive"] }
# serde_json = "1.0"
# tokio = { version = "1", features = ["full"] }
# async-trait = "0.1"

# Rate limiting
governor = "0.6"

# Audit logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }

[features]
mcp = ["rust-mcp-sdk"]  # Optional: compile MCP server support
```

---

## Implementation Plan

### Phase 1: MCP Server Container (Days 1-3)

**Tasks:**
1. Create `Dockerfile.mcp` with multi-stage build
2. Add MCP feature flag to Cargo.toml
3. Implement container-aware host detection
4. Test local container build and MCP communication

**Verification:**
```bash
# Build image
docker build -f Dockerfile.mcp -t jarvy-mcp:local .

# Test MCP initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | docker run -i jarvy-mcp:local

# Test tool listing
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | docker run -i jarvy-mcp:local
```

### Phase 2: Registry Metadata (Days 4-5)

**Tasks:**
1. Create `mcp/server.yaml` manifest
2. Create `mcp/tools.json` definitions
3. Create `mcp/readme.md` documentation
4. Create Jarvy icon asset (256x256 PNG)
5. Validate YAML syntax with Docker's tools

**Validation:**
```bash
# Use Docker MCP registry tools to validate
git clone https://github.com/docker/mcp-registry.git
cd mcp-registry
task validate -- --server /path/to/jarvy/mcp
```

### Phase 3: Docker MCP Registry Submission (Days 6-8)

**Tasks:**
1. Fork `docker/mcp-registry`
2. Run registry wizard to generate configuration
3. Test locally with Docker Desktop MCP Toolkit
4. Submit pull request
5. Address review feedback

**Submission Process:**
```bash
# Clone and setup
git clone https://github.com/docker/mcp-registry.git
cd mcp-registry
git checkout -b add-jarvy

# Use wizard or manual setup
task wizard
# OR
task create -- --category devops https://github.com/jarvy-dev/jarvy

# Test locally
task build -- --tools jarvy
task catalog -- jarvy

# Import to Docker Desktop for testing
docker mcp catalog import $PWD/catalogs/jarvy/catalog.yaml

# Submit PR
git add servers/jarvy/
git commit -m "Add Jarvy MCP server for safe tool installation"
git push origin add-jarvy
gh pr create --title "Add Jarvy - Safe Tool Installation MCP Server"
```

### Phase 4: CI/CD Automation (Days 9-10)

**Tasks:**
1. Create GitHub Actions workflow for MCP registry updates
2. Test workflow with manual dispatch
3. Document release process

**Workflow:** `.github/workflows/mcp-registry.yml`
```yaml
name: Update Docker MCP Registry

on:
  release:
    types: [published]
  workflow_dispatch:

jobs:
  update-mcp-registry:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Get release info
        id: release
        run: |
          echo "version=${GITHUB_REF#refs/tags/v}" >> $GITHUB_OUTPUT
          echo "commit=${GITHUB_SHA}" >> $GITHUB_OUTPUT

      - name: Create MCP Registry PR
        env:
          GH_TOKEN: ${{ secrets.MCP_REGISTRY_PAT }}
        run: |
          git clone https://github.com/docker/mcp-registry.git
          cd mcp-registry
          git checkout -b jarvy-v${{ steps.release.outputs.version }}

          # Update commit reference in server.yaml
          mkdir -p servers/jarvy
          cp ../mcp/server.yaml servers/jarvy/
          cp ../mcp/tools.json servers/jarvy/
          cp ../mcp/readme.md servers/jarvy/

          # Update source reference
          sed -i 's/commit:.*/commit: ${{ steps.release.outputs.commit }}/' servers/jarvy/server.yaml

          git add servers/jarvy/
          git commit -m "Update jarvy to v${{ steps.release.outputs.version }}"
          git push origin jarvy-v${{ steps.release.outputs.version }}

          gh pr create \
            --repo docker/mcp-registry \
            --title "Update jarvy to v${{ steps.release.outputs.version }}" \
            --body "Automated update for Jarvy release v${{ steps.release.outputs.version }}"
```

---

## Security Considerations

### Container Security Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Security Layers                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  1. Container Isolation                                               │
│     ├── Non-root user (uid 1000)                                     │
│     ├── Read-only filesystem (--read-only supported)                 │
│     ├── No network access required                                   │
│     └── Minimal attack surface (Alpine base)                         │
│                                                                       │
│  2. MCP Protocol Safety                                               │
│     ├── Dry-run by default for all installs                         │
│     ├── Rate limiting (3 installs/minute max)                       │
│     ├── Allowlist/denylist configuration                            │
│     └── Audit logging of all operations                             │
│                                                                       │
│  3. Host Interaction Safety                                           │
│     ├── Tool checks: read-only path inspection                      │
│     ├── Installations: user confirmation required                   │
│     ├── Package managers: delegated to host                         │
│     └── No direct host filesystem modification                      │
│                                                                       │
│  4. Docker-Built Image Benefits                                       │
│     ├── Cryptographic signatures (image signing)                    │
│     ├── Provenance attestation (build verification)                 │
│     ├── SBOM generation (dependency tracking)                       │
│     └── Automatic security updates (base image rebuilds)            │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### Host Access Strategy

The containerized MCP server needs to interact with the host system. Docker Desktop's MCP integration handles this through:

1. **Tool Detection**: Docker Desktop mounts necessary host paths for tool detection
2. **Installation Execution**: Install commands are communicated back to the MCP client, which executes them on the host
3. **No Direct Host Access**: Container never directly modifies host filesystem

```
MCP Client (Claude Desktop)
    │
    │ 1. "Install ripgrep"
    ▼
Jarvy Container
    │
    │ 2. Generates: "brew install ripgrep"
    │    Returns as tool result
    ▼
MCP Client
    │
    │ 3. Shows to user, requests confirmation
    │ 4. Executes on HOST (not in container)
    ▼
Host System
    │
    │ 5. Homebrew installs ripgrep
    ▼
Success response to user
```

### Audit Logging

All MCP operations are logged:

```json
{
  "timestamp": "2025-01-17T10:30:00Z",
  "event": "tool_call",
  "tool": "jarvy_install_tool",
  "params": {"name": "ripgrep", "dry_run": false},
  "result": "success",
  "client": "claude-desktop",
  "duration_ms": 45
}
```

---

## Testing Strategy

### Local Testing

```bash
# 1. Build the image
docker build -f Dockerfile.mcp -t jarvy-mcp:local .

# 2. Test MCP protocol compliance
cat <<EOF | docker run -i jarvy-mcp:local
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"jarvy_list_tools","arguments":{}}}
EOF

# 3. Test with MCP Inspector
npx @anthropic-ai/mcp-inspector docker run -i jarvy-mcp:local
```

### Docker MCP Registry Testing

```bash
# Clone registry and validate
git clone https://github.com/docker/mcp-registry.git
cd mcp-registry

# Copy Jarvy files
mkdir -p servers/jarvy
cp /path/to/jarvy/mcp/* servers/jarvy/

# Build and generate catalog
task build -- --tools jarvy
task catalog -- jarvy

# Import to Docker Desktop for testing
docker mcp catalog import $PWD/catalogs/jarvy/catalog.yaml
```

### Integration Testing

Test with actual MCP clients:

1. **Claude Desktop**: Add to `claude_desktop_config.json`, test tool discovery and installation
2. **Cursor**: Add to MCP settings, verify tool suggestions work
3. **MCP Inspector**: Comprehensive protocol testing

---

## Directory Structure

```
jarvy/
├── Dockerfile.mcp                    # MCP server container build
├── mcp/
│   ├── server.yaml                   # Docker MCP registry manifest
│   ├── tools.json                    # MCP tool definitions
│   └── readme.md                     # MCP-specific documentation
├── assets/
│   └── jarvy-icon.png               # 256x256 icon for MCP catalog
├── src/
│   └── mcp/                         # MCP server implementation (PRD-021)
├── .github/
│   └── workflows/
│       └── mcp-registry.yml         # Auto-update workflow
└── docs/
    └── mcp/
        ├── README.md                # Full MCP documentation
        └── docker-desktop.md        # Docker Desktop setup guide
```

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Listed in Docker Hub MCP catalog | Yes | hub.docker.com/mcp shows jarvy |
| Docker Desktop one-click install | Yes | MCP Toolkit enables jarvy |
| Image size | < 20MB | `docker images mcp/jarvy` |
| Startup time | < 200ms | Time to first MCP response |
| MCP protocol compliance | 100% | MCP Inspector tests pass |
| PR approval time | < 2 weeks | Docker team review completion |
| Zero security vulnerabilities | 0 critical/high | Trivy/Snyk scan |

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Docker rejects PR | Low | High | Follow contributing guide exactly, test thoroughly |
| Host tool detection fails in container | Medium | High | Document supported configurations, test all platforms |
| MCP protocol version changes | Low | Medium | Pin to stable version, monitor spec updates |
| Large image size | Low | Low | Multi-stage build, Alpine base, strip binary |
| Rate limiting in Docker builds | Low | Low | Cache layers, minimal dependencies |

---

## Open Questions

1. **Host Integration Method**: How does Docker Desktop's MCP Toolkit handle host tool execution?
   - Need to test with actual Docker Desktop integration
   - May require special documentation for manual setups

2. **Update Frequency**: How often does Docker rebuild MCP catalog images?
   - On merge to main?
   - On schedule (daily/weekly)?
   - Need to confirm with Docker team

3. **Platform Support**: Can single container support all host platforms?
   - Linux container detecting macOS/Windows host tools
   - May need platform-specific handling

---

## References

- [Docker MCP Registry](https://github.com/docker/mcp-registry)
- [Docker MCP Registry CONTRIBUTING.md](https://github.com/docker/mcp-registry/blob/main/CONTRIBUTING.md)
- [Docker Hub MCP Catalog](https://hub.docker.com/mcp)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk) - Rust MCP SDK
- [PRD-021: Jarvy MCP Server](./021-mcp-server.md) - MCP implementation details
- [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) - Example MCP servers
