# PRD-023: Docker Desktop MCP Catalog Distribution

## Status: Draft
## Priority: P1
## Effort: Small (2-3 days)
## Depends On: PRD-021 (MCP Server Implementation)

---

## Problem Statement

PRD-021 defines the Jarvy MCP server implementation, but doesn't address distribution via Docker Desktop's MCP catalog. Docker Desktop's MCP Toolkit provides:

1. **One-click installation** for users discovering MCP servers
2. **Automatic updates** when new versions are published
3. **Enhanced security** - Docker-built images include signatures, provenance, SBOMs
4. **Enterprise visibility** - IT can see/approve MCP servers used by developers
5. **Discoverability** - Listed in Docker Hub's MCP catalog (hub.docker.com/mcp)

To reach the largest audience of developers using Claude Desktop, Cursor, and other MCP clients, Jarvy needs to be in this catalog.

---

## Goals

1. **Containerize** the Jarvy MCP server for Docker-based distribution
2. **Submit to Docker MCP Registry** via pull request to `docker/mcp-registry`
3. **Enable one-click install** via Docker Desktop MCP Toolkit
4. **Automatic publishing** on new Jarvy releases

---

## Non-Goals

- Hosting our own Docker images (Docker will build and host in `mcp/jarvy`)
- SSE/HTTP transport (stdio only for Docker MCP catalog)
- Remote server deployment (local containerized server only)

---

## Requirements

### Docker MCP Registry Requirements

Per [docker/mcp-registry CONTRIBUTING.md](https://github.com/docker/mcp-registry/blob/main/CONTRIBUTING.md):

| Requirement | Status |
|-------------|--------|
| MIT or Apache 2.0 license | ✅ Jarvy is MIT |
| Dockerfile in repo | ❌ Need to create |
| `server.yaml` manifest | ❌ Need to create |
| `tools.json` (optional) | ❌ Need to create |
| MCP server lists tools on startup | ✅ Will be implemented in PRD-021 |

### Deliverables

1. **`Dockerfile.mcp`** - Multi-stage build for minimal MCP server image
2. **`mcp/server.yaml`** - Docker MCP registry manifest
3. **`mcp/tools.json`** - Pre-defined tool listing (avoids runtime tool listing)
4. **GitHub Actions workflow** - Trigger Docker MCP registry rebuild on release

---

## Technical Design

### 1. Dockerfile

```dockerfile
# Dockerfile.mcp
# Multi-stage build for minimal Jarvy MCP server image

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf

WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -rf src

# Build actual binary
COPY . .
RUN cargo build --release --bin jarvy

# Verify binary
RUN ./target/release/jarvy --version

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM alpine:3.21

# Install runtime dependencies for package manager detection
# (Jarvy needs to detect available package managers on the host)
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN addgroup -S jarvy && adduser -S jarvy -G jarvy

# Copy binary
COPY --from=builder /build/target/release/jarvy /usr/local/bin/jarvy

# Switch to non-root user
USER jarvy

# MCP servers use stdio transport
ENTRYPOINT ["jarvy", "mcp"]

# Labels for container metadata
LABEL org.opencontainers.image.title="Jarvy MCP Server"
LABEL org.opencontainers.image.description="Safe cross-platform development tool installation for LLMs"
LABEL org.opencontainers.image.vendor="Jarvy"
LABEL org.opencontainers.image.source="https://github.com/jarvy-dev/jarvy"
LABEL org.opencontainers.image.licenses="MIT"
```

### 2. server.yaml (MCP Registry Manifest)

```yaml
# mcp/server.yaml
# Docker MCP Registry manifest for Jarvy

name: jarvy
image: mcp/jarvy
type: server

meta:
  category: devops
  tags:
    - devtools
    - installation
    - package-manager
    - cross-platform
    - developer-experience

about:
  title: Jarvy - Safe Tool Installation
  description: |
    Install development tools safely across macOS, Linux, and Windows.
    Jarvy provides LLMs with accurate, platform-specific installation
    commands and verification, eliminating hallucinated package names.

    Supports 100+ tools including: git, docker, node, python, rust, go,
    kubectl, terraform, and more.
  icon: https://raw.githubusercontent.com/jarvy-dev/jarvy/main/assets/icon.png

source:
  project: https://github.com/jarvy-dev/jarvy
  # branch and commit are auto-populated by Docker's build system
  dockerfile: Dockerfile.mcp

config:
  description: |
    Jarvy MCP server configuration. Most users don't need any configuration.

    Optional: Set tool allowlist/denylist for enterprise environments.

  # No secrets required - Jarvy uses local package managers
  # Optional environment variables for enterprise configuration
  env:
    - name: JARVY_MCP_ALLOWLIST
      example: "git,docker,node,python"
      description: "Comma-separated list of tools to allow (empty = all)"
    - name: JARVY_MCP_DENYLIST
      example: "brew"
      description: "Comma-separated list of tools to deny"
    - name: JARVY_MCP_REQUIRE_CONFIRMATION
      example: "true"
      description: "Require user confirmation for installs (default: true)"

# Optional: Define parameters that appear in Docker Desktop UI
  parameters:
    type: object
    properties:
      allowlist:
        type: string
        description: "Comma-separated list of allowed tools (empty = all)"
      require_confirmation:
        type: boolean
        default: true
        description: "Require confirmation before installing tools"
```

### 3. tools.json (Pre-defined Tool Listing)

The `tools.json` file prevents build failures when the MCP server can't list tools at build time (e.g., when configuration is required).

```json
[
  {
    "name": "jarvy_list_tools",
    "description": "List all development tools Jarvy can install, with optional filtering by category or platform",
    "inputSchema": {
      "type": "object",
      "properties": {
        "category": {
          "type": "string",
          "enum": ["language", "database", "container", "cli", "editor", "kubernetes", "all"],
          "description": "Filter by tool category"
        },
        "platform": {
          "type": "string",
          "enum": ["macos", "linux", "windows", "current"],
          "description": "Filter by platform support (default: current)"
        },
        "search": {
          "type": "string",
          "description": "Search tools by name"
        }
      }
    }
  },
  {
    "name": "jarvy_get_tool",
    "description": "Get detailed information about a specific tool including installation methods for all platforms",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "description": "Tool name (e.g., 'git', 'docker', 'node')"
        }
      },
      "required": ["name"]
    }
  },
  {
    "name": "jarvy_check_tool",
    "description": "Check if a tool is installed and get its version",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "description": "Tool name to check"
        }
      },
      "required": ["name"]
    }
  },
  {
    "name": "jarvy_check_multiple",
    "description": "Check installation status of multiple tools at once",
    "inputSchema": {
      "type": "object",
      "properties": {
        "tools": {
          "type": "array",
          "items": { "type": "string" },
          "description": "List of tool names to check"
        }
      },
      "required": ["tools"]
    }
  },
  {
    "name": "jarvy_install_tool",
    "description": "Install a development tool. By default returns a preview (dry_run=true). Set dry_run=false to execute installation with user confirmation.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "description": "Tool name to install"
        },
        "version": {
          "type": "string",
          "description": "Version hint (default: 'latest')"
        },
        "dry_run": {
          "type": "boolean",
          "description": "Preview installation without executing (default: true)",
          "default": true
        }
      },
      "required": ["name"]
    }
  },
  {
    "name": "jarvy_platform_info",
    "description": "Get information about the current platform, OS version, and available package managers",
    "inputSchema": {
      "type": "object",
      "properties": {}
    }
  }
]
```

### 4. GitHub Actions Workflow

```yaml
# .github/workflows/mcp-registry.yml
name: Update Docker MCP Registry

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      trigger_rebuild:
        description: 'Trigger MCP registry rebuild'
        required: false
        default: 'true'

jobs:
  notify-mcp-registry:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Get release info
        id: release
        run: |
          echo "version=${GITHUB_REF#refs/tags/}" >> $GITHUB_OUTPUT
          echo "commit=${GITHUB_SHA}" >> $GITHUB_OUTPUT

      # Option 1: Create PR to docker/mcp-registry (manual review)
      - name: Create MCP Registry PR
        if: github.event_name == 'release'
        env:
          GH_TOKEN: ${{ secrets.MCP_REGISTRY_PAT }}
        run: |
          # Clone mcp-registry
          git clone https://github.com/docker/mcp-registry.git
          cd mcp-registry

          # Create branch
          git checkout -b jarvy-${{ steps.release.outputs.version }}

          # Update server.yaml with new commit
          mkdir -p servers/jarvy
          cp ../mcp/server.yaml servers/jarvy/
          cp ../mcp/tools.json servers/jarvy/

          # Update commit reference
          yq -i '.source.commit = "${{ steps.release.outputs.commit }}"' servers/jarvy/server.yaml
          yq -i '.source.branch = "${{ steps.release.outputs.version }}"' servers/jarvy/server.yaml

          # Commit and push
          git add servers/jarvy/
          git commit -m "Update jarvy to ${{ steps.release.outputs.version }}"
          git push origin jarvy-${{ steps.release.outputs.version }}

          # Create PR
          gh pr create \
            --title "Update jarvy to ${{ steps.release.outputs.version }}" \
            --body "Automated update for Jarvy release ${{ steps.release.outputs.version }}" \
            --base main

      # Option 2: Trigger rebuild via workflow dispatch (if Docker provides this)
      - name: Trigger MCP rebuild
        if: github.event.inputs.trigger_rebuild == 'true'
        run: |
          echo "Docker MCP registry rebuild would be triggered here"
          # Future: Docker may provide an API or webhook for this
```

---

## Directory Structure

```
jarvy/
├── Dockerfile.mcp              # MCP server container build
├── mcp/
│   ├── server.yaml             # Docker MCP registry manifest
│   ├── tools.json              # Pre-defined MCP tools
│   └── README.md               # MCP-specific documentation
├── .github/
│   └── workflows/
│       └── mcp-registry.yml    # Auto-update workflow
└── docs/
    └── mcp/
        └── docker-desktop.md   # Docker Desktop setup guide
```

---

## Implementation Plan

### Phase 1: Create Container Assets (Day 1)

1. Create `Dockerfile.mcp` with multi-stage build
2. Test local build: `docker build -f Dockerfile.mcp -t jarvy-mcp .`
3. Test MCP server in container: `docker run -i jarvy-mcp`
4. Verify tool listing works

**Verification:**
```bash
# Build image
docker build -f Dockerfile.mcp -t jarvy-mcp .

# Test MCP initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | docker run -i jarvy-mcp

# Test tool listing
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | docker run -i jarvy-mcp
```

### Phase 2: Create Registry Manifest (Day 1)

1. Create `mcp/server.yaml` with metadata
2. Create `mcp/tools.json` with all MCP tools
3. Validate YAML syntax
4. Create icon asset if needed

### Phase 3: Submit to Docker MCP Registry (Day 2)

1. Fork `docker/mcp-registry`
2. Run `task wizard` or `task create` to validate
3. Test locally with Docker Desktop:
   ```bash
   task build -- --tools jarvy
   task catalog -- jarvy
   docker mcp catalog import $PWD/catalogs/jarvy/catalog.yaml
   ```
4. Verify in Docker Desktop MCP Toolkit
5. Submit PR to `docker/mcp-registry`
6. Respond to review feedback

### Phase 4: Automate Updates (Day 3)

1. Create GitHub Actions workflow
2. Test workflow with manual dispatch
3. Document release process

---

## Testing Strategy

### Local Testing

```bash
# 1. Build the image
docker build -f Dockerfile.mcp -t jarvy-mcp:local .

# 2. Test basic MCP communication
docker run -i jarvy-mcp:local < test-requests.jsonl

# 3. Test with MCP Inspector
npx @anthropic-ai/mcp-inspector docker run -i jarvy-mcp:local
```

### Docker MCP Registry Testing

```bash
# Clone and set up mcp-registry
git clone https://github.com/docker/mcp-registry.git
cd mcp-registry

# Copy Jarvy files
mkdir -p servers/jarvy
cp /path/to/jarvy/mcp/server.yaml servers/jarvy/
cp /path/to/jarvy/mcp/tools.json servers/jarvy/

# Build and test
task build -- --tools jarvy
task catalog -- jarvy

# Import into Docker Desktop
docker mcp catalog import $PWD/catalogs/jarvy/catalog.yaml

# Test in Docker Desktop MCP Toolkit UI
# 1. Open Docker Desktop
# 2. Go to MCP Toolkit
# 3. Find "jarvy" in catalog
# 4. Enable and configure
# 5. Test with Claude Desktop
```

### Integration Testing with Claude Desktop

```json
// Test configuration in Claude Desktop
{
  "mcpServers": {
    "jarvy": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "mcp/jarvy"]
    }
  }
}
```

Test prompts:
1. "What tools can Jarvy install?"
2. "Is git installed on my system?"
3. "Install ripgrep for me" (should show dry-run first)

---

## Security Considerations

### Container Security

1. **Non-root user**: Container runs as `jarvy` user, not root
2. **Minimal base image**: Alpine Linux (~5MB)
3. **No network access needed**: MCP uses stdio, not network
4. **Read-only filesystem**: Can run with `--read-only` flag

### Docker-Built Image Benefits

When Docker builds the image (vs self-hosted):
- **Cryptographic signatures**: Image is signed by Docker
- **Provenance tracking**: Build provenance attestation
- **SBOM generation**: Software Bill of Materials included
- **Automatic security updates**: Rebuilt on base image updates

### Host Access Considerations

The MCP server needs to:
1. **Execute package manager commands** on the host (brew, apt, etc.)
2. **Check installed tool versions** via command execution

This means the container needs:
```bash
# Mount host package managers (example for macOS)
docker run -i \
  -v /usr/local/bin:/usr/local/bin:ro \
  -v /opt/homebrew:/opt/homebrew:ro \
  mcp/jarvy
```

**Important**: Document this clearly - the container can't install tools inside itself; it installs on the host system.

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Listed in Docker Hub MCP catalog | Yes | hub.docker.com/mcp shows jarvy |
| Docker Desktop install works | Yes | One-click enable in MCP Toolkit |
| Image size | < 15MB | `docker images mcp/jarvy` |
| Startup time | < 500ms | Time to first MCP response |
| PR approval time | < 1 week | Docker team review |

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Docker rejects PR | Low | High | Follow contributing guide exactly, test locally first |
| Container can't access host tools | Medium | High | Document volume mounts clearly, test on all platforms |
| MCP protocol version mismatch | Low | Medium | Pin to stable protocol version |
| Large image size | Low | Low | Multi-stage build, Alpine base |

---

## Open Questions

1. **Volume mounts**: How should users mount their package managers into the container?
   - Option A: Document manual mounts
   - Option B: Docker Desktop handles this automatically for MCP servers

2. **Host tool execution**: Can containerized MCP server run `brew install` on host?
   - Need to test with Docker Desktop's MCP integration
   - May need special handling or documentation

3. **Update frequency**: How often does Docker rebuild MCP images?
   - On every commit to `docker/mcp-registry`?
   - On schedule?

---

## References

- [Docker MCP Registry](https://github.com/docker/mcp-registry)
- [Docker MCP Registry Contributing Guide](https://github.com/docker/mcp-registry/blob/main/CONTRIBUTING.md)
- [Docker Hub MCP Catalog](https://hub.docker.com/mcp)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [PRD-021: Jarvy MCP Server](./021-mcp-server.md)
- [Example: GitHub MCP Server](https://github.com/docker/mcp-registry/tree/main/servers/github)
