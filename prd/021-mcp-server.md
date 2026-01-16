# PRD-021: Jarvy MCP Server for Safe Tool Installation

## Overview

Expose Jarvy as a Model Context Protocol (MCP) server, enabling LLMs (Claude, GPT, etc.) to safely discover, verify, and install development tools. The MCP server provides structured tool metadata, installation instructions, and controlled execution with mandatory user confirmation.

## Problem Statement

LLMs frequently need to help users install development tools but face several challenges:

1. **Hallucinated commands**: LLMs often generate incorrect or outdated installation commands
2. **Platform blindness**: LLMs don't know the user's OS, available package managers, or installed tools
3. **No verification**: LLMs can't verify if tools are already installed or if installation succeeded
4. **Safety concerns**: Arbitrary shell command execution from LLMs is dangerous
5. **Inconsistent UX**: Every LLM integration reinvents tool installation differently

Jarvy already solves cross-platform tool installation. Exposing it via MCP creates a **safe, standardized interface** for LLMs to leverage this expertise.

## Goals

1. **Safe by default**: All installations require explicit user confirmation
2. **Accurate metadata**: LLMs get correct, platform-specific installation info
3. **Verification**: LLMs can check if tools are installed before/after installation
4. **Discoverability**: LLMs can search and explore available tools
5. **Wide distribution**: Publish to npm, Docker Hub, and MCP registries
6. **Zero configuration**: Works out-of-the-box with Claude Desktop, Cursor, VS Code, etc.

## Non-Goals

- Automatic/unattended installations (always require confirmation)
- Managing tool configurations beyond default hooks
- Version pinning or rollback capabilities (v1)
- Installing tools not in Jarvy's registry

## Target MCP Clients

| Client | Priority | Transport | Notes |
|--------|----------|-----------|-------|
| Claude Desktop | P0 | stdio | Primary target |
| Claude Code | P0 | stdio | CLI integration |
| Cursor | P0 | stdio | IDE integration |
| VS Code + Continue | P1 | stdio | Extension |
| Zed | P1 | stdio | Editor |
| Custom agents | P2 | stdio/SSE | SDK users |

## Requirements

### Functional Requirements

#### FR-1: MCP Tool Interface

Expose the following MCP tools:

```typescript
// Tool definitions (MCP schema format)
tools: [
  {
    name: "jarvy_list_tools",
    description: "List all tools Jarvy can install, with optional filtering",
    inputSchema: {
      type: "object",
      properties: {
        category: {
          type: "string",
          enum: ["language", "database", "container", "cli", "editor", "all"],
          description: "Filter by tool category"
        },
        platform: {
          type: "string",
          enum: ["macos", "linux", "windows", "current"],
          description: "Filter by platform support (default: current)"
        },
        search: {
          type: "string",
          description: "Search tools by name or description"
        }
      }
    }
  },
  {
    name: "jarvy_get_tool",
    description: "Get detailed information about a specific tool",
    inputSchema: {
      type: "object",
      properties: {
        name: { type: "string", description: "Tool name (e.g., 'git', 'docker', 'node')" }
      },
      required: ["name"]
    }
  },
  {
    name: "jarvy_check_tool",
    description: "Check if a tool is installed and get its version",
    inputSchema: {
      type: "object",
      properties: {
        name: { type: "string", description: "Tool name to check" }
      },
      required: ["name"]
    }
  },
  {
    name: "jarvy_install_tool",
    description: "Install a development tool (requires user confirmation)",
    inputSchema: {
      type: "object",
      properties: {
        name: { type: "string", description: "Tool name to install" },
        version: { type: "string", description: "Version hint (default: 'latest')" },
        dry_run: { type: "boolean", description: "Preview installation without executing (default: true)" }
      },
      required: ["name"]
    }
  },
  {
    name: "jarvy_check_multiple",
    description: "Check installation status of multiple tools at once",
    inputSchema: {
      type: "object",
      properties: {
        tools: {
          type: "array",
          items: { type: "string" },
          description: "List of tool names to check"
        }
      },
      required: ["tools"]
    }
  }
]
```

#### FR-2: MCP Resource Interface

Expose the following MCP resources:

```typescript
resources: [
  {
    uri: "jarvy://tools/index",
    name: "Tool Index",
    description: "Complete index of all supported tools with metadata",
    mimeType: "application/json"
  },
  {
    uri: "jarvy://platform/info",
    name: "Platform Info",
    description: "Current platform, OS version, and available package managers",
    mimeType: "application/json"
  },
  {
    uri: "jarvy://tools/{name}",
    name: "Tool Details",
    description: "Detailed information about a specific tool",
    mimeType: "application/json"
  }
]
```

#### FR-3: Safety Mechanisms

1. **Dry-run by default**: `jarvy_install_tool` defaults to `dry_run: true`
2. **Confirmation prompts**: Non-dry-run installs show confirmation via MCP sampling
3. **Rate limiting**: Max 10 tool checks per minute, 3 installs per minute
4. **Audit logging**: All MCP requests logged to `~/.jarvy/mcp-audit.log`
5. **Tool allowlist/denylist**: Configurable via `~/.jarvy/mcp-config.toml`

```toml
# ~/.jarvy/mcp-config.toml
[mcp]
# If set, only these tools can be installed via MCP
allowlist = ["git", "docker", "node", "python"]

# These tools are never installable via MCP (takes precedence over allowlist)
denylist = ["brew"]  # Don't let MCP install package managers

# Require confirmation for all installs (default: true)
require_confirmation = true

# Rate limits
max_checks_per_minute = 10
max_installs_per_minute = 3

# Audit logging
audit_log = "~/.jarvy/mcp-audit.log"
```

#### FR-4: Response Formats

Tool responses should be structured for LLM consumption:

```json
// jarvy_list_tools response
{
  "tools": [
    {
      "name": "git",
      "description": "Distributed version control system",
      "category": "cli",
      "platforms": ["macos", "linux", "windows"],
      "has_default_hook": false
    }
  ],
  "count": 85,
  "platform": "macos"
}

// jarvy_get_tool response
{
  "name": "docker",
  "command": "docker",
  "description": "Container runtime and tooling",
  "category": "container",
  "current_platform": {
    "os": "macos",
    "install_method": "cask",
    "package_name": "docker",
    "package_manager": "homebrew"
  },
  "all_platforms": {
    "macos": { "cask": "docker" },
    "linux": { "apt": "docker.io", "dnf": "docker" },
    "windows": { "winget": "Docker.DockerDesktop" }
  },
  "default_hook": {
    "description": "Add user to docker group on Linux",
    "platform": "linux"
  },
  "documentation_url": "https://docs.docker.com/get-docker/"
}

// jarvy_check_tool response
{
  "name": "git",
  "installed": true,
  "version": "2.43.0",
  "satisfies": "latest",
  "path": "/usr/bin/git"
}

// jarvy_install_tool response (dry_run: true)
{
  "name": "ripgrep",
  "dry_run": true,
  "would_execute": {
    "command": "brew install ripgrep",
    "package_manager": "homebrew",
    "requires_sudo": false
  },
  "notes": "Set dry_run to false and confirm to proceed with installation"
}

// jarvy_install_tool response (dry_run: false, after confirmation)
{
  "name": "ripgrep",
  "success": true,
  "installed_version": "14.1.0",
  "duration_ms": 3420,
  "hook_executed": false
}
```

#### FR-5: MCP Prompts

Provide helpful prompts for common workflows:

```typescript
prompts: [
  {
    name: "setup_dev_environment",
    description: "Interactive workflow to set up a development environment",
    arguments: [
      {
        name: "project_type",
        description: "Type of project (e.g., 'rust', 'node', 'python', 'go')",
        required: true
      }
    ]
  },
  {
    name: "diagnose_missing_tools",
    description: "Check which common tools are missing and suggest installations"
  }
]
```

### Non-Functional Requirements

1. **Startup time**: MCP server should start in < 100ms
2. **Memory**: < 20MB RSS at idle
3. **Binary size**: < 10MB for standalone MCP server
4. **Compatibility**: MCP protocol version 2024-11-05 or later
5. **Error handling**: All errors return structured MCP error responses

## Architecture

### Binary Structure

```
jarvy/
├── src/
│   ├── main.rs           # CLI entry point (existing)
│   └── mcp/
│       ├── mod.rs        # MCP module root
│       ├── server.rs     # MCP server implementation
│       ├── tools.rs      # Tool handlers (list, get, check, install)
│       ├── resources.rs  # Resource handlers
│       ├── prompts.rs    # Prompt handlers
│       ├── safety.rs     # Rate limiting, allowlist, confirmation
│       └── config.rs     # MCP-specific configuration
└── Cargo.toml
```

### MCP Server Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     MCP Client (Claude Desktop)                   │
├─────────────────────────────────────────────────────────────────┤
│  User: "Install ripgrep for me"                                  │
│  Claude: [calls jarvy_check_tool(name: "ripgrep")]              │
│  Claude: [calls jarvy_install_tool(name: "ripgrep", dry_run: true)]│
│  Claude: "ripgrep would be installed via `brew install ripgrep`. │
│           Should I proceed?"                                      │
│  User: "Yes"                                                      │
│  Claude: [calls jarvy_install_tool(name: "ripgrep", dry_run: false)]│
│  → MCP server shows confirmation prompt                           │
│  → User confirms in terminal                                      │
│  Claude: "ripgrep v14.1.0 installed successfully!"               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Jarvy MCP Server (stdio)                      │
├─────────────────────────────────────────────────────────────────┤
│  1. Receive JSON-RPC request                                     │
│  2. Validate against allowlist/denylist                          │
│  3. Check rate limits                                            │
│  4. Execute tool handler                                         │
│     - list_tools: Query tool registry                           │
│     - check_tool: Run `command --version`                       │
│     - install_tool (dry_run): Return install command            │
│     - install_tool (execute): Show confirmation, run installer  │
│  5. Log to audit file                                           │
│  6. Return JSON-RPC response                                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Jarvy Tool Registry (existing)                 │
├─────────────────────────────────────────────────────────────────┤
│  - ToolSpec definitions                                          │
│  - Platform-specific install methods                             │
│  - Custom installers (nvm, rustup, brew)                        │
│  - Default hooks                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Confirmation Flow (Non-Dry-Run Install)

```
┌─────────────────────────────────────────────────────────────────┐
│  MCP Request: install_tool(name: "docker", dry_run: false)       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Safety Check                                                     │
│  ├── Is "docker" in allowlist? (if allowlist configured)        │
│  ├── Is "docker" in denylist?                                   │
│  ├── Rate limit check (< 3 installs/minute)                     │
│  └── require_confirmation = true?                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Confirmation Prompt (via stderr to terminal)                     │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Jarvy MCP: Install docker?                                   ││
│  │                                                              ││
│  │ This will execute:                                           ││
│  │   brew install --cask docker                                 ││
│  │                                                              ││
│  │ Requested by: Claude Desktop (MCP client)                    ││
│  │                                                              ││
│  │ [Y]es / [N]o / [A]lways allow docker:                       ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                    User types 'y' + Enter
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Execute Installation                                            │
│  ├── Run: brew install --cask docker                            │
│  ├── Capture stdout/stderr                                       │
│  ├── Verify installation: docker --version                      │
│  └── Run default hook if applicable                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Audit Log Entry                                                  │
│  {                                                                │
│    "timestamp": "2024-01-15T10:30:00Z",                          │
│    "action": "install",                                          │
│    "tool": "docker",                                             │
│    "success": true,                                              │
│    "version": "24.0.7",                                          │
│    "duration_ms": 45000,                                         │
│    "client": "claude-desktop"                                    │
│  }                                                                │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Core MCP Server (Week 1)

#### 1.1 MCP Protocol Implementation

```rust
// src/mcp/server.rs
use async_std::io::{stdin, stdout};
use serde::{Deserialize, Serialize};

/// MCP server using stdio transport
pub struct JarvyMcpServer {
    config: McpConfig,
    rate_limiter: RateLimiter,
    audit_log: AuditLog,
}

impl JarvyMcpServer {
    pub async fn run(&self) -> Result<(), McpError> {
        let stdin = stdin();
        let stdout = stdout();

        // JSON-RPC message loop
        loop {
            let request = self.read_request(&stdin).await?;
            let response = self.handle_request(request).await;
            self.write_response(&stdout, response).await?;
        }
    }

    async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(req).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(req).await,
            "prompts/list" => self.handle_prompts_list().await,
            "prompts/get" => self.handle_prompts_get(req).await,
            _ => JsonRpcResponse::error(req.id, "Method not found"),
        }
    }
}
```

#### 1.2 Tool Handlers

```rust
// src/mcp/tools.rs
use crate::tools::spec::{generate_tool_index, get_tool_spec, list_tool_names};

pub async fn handle_list_tools(params: ListToolsParams) -> McpResult<ListToolsResponse> {
    let index = generate_tool_index();

    let tools: Vec<ToolSummary> = index.tools
        .iter()
        .filter(|t| matches_filter(t, &params))
        .map(|t| ToolSummary {
            name: t.name.clone(),
            description: get_tool_description(&t.name),
            category: categorize_tool(&t.name),
            platforms: get_supported_platforms(t),
            has_default_hook: has_default_hook(&t.name),
        })
        .collect();

    Ok(ListToolsResponse {
        tools,
        count: tools.len(),
        platform: current_platform(),
    })
}

pub async fn handle_check_tool(params: CheckToolParams) -> McpResult<CheckToolResponse> {
    let spec = get_tool_spec(&params.name)
        .ok_or_else(|| McpError::unknown_tool(&params.name))?;

    let version = get_installed_version(spec.command);

    Ok(CheckToolResponse {
        name: params.name,
        installed: version.is_some(),
        version,
        satisfies: "latest".to_string(),
        path: which::which(spec.command).ok().map(|p| p.display().to_string()),
    })
}

pub async fn handle_install_tool(
    params: InstallToolParams,
    config: &McpConfig,
    rate_limiter: &RateLimiter,
) -> McpResult<InstallToolResponse> {
    // Safety checks
    check_allowlist(&params.name, config)?;
    check_denylist(&params.name, config)?;
    rate_limiter.check_install_limit()?;

    let spec = get_tool_spec(&params.name)
        .ok_or_else(|| McpError::unknown_tool(&params.name))?;

    let install_info = get_install_info_for_current_platform(spec);

    if params.dry_run.unwrap_or(true) {
        return Ok(InstallToolResponse::DryRun {
            name: params.name,
            would_execute: install_info,
            notes: "Set dry_run to false and confirm to proceed".to_string(),
        });
    }

    // Require confirmation for actual install
    if config.require_confirmation {
        let confirmed = prompt_user_confirmation(&params.name, &install_info)?;
        if !confirmed {
            return Err(McpError::user_cancelled());
        }
    }

    // Execute installation
    let start = Instant::now();
    spec.ensure(&params.version.unwrap_or("latest".to_string()))?;

    let installed_version = get_installed_version(spec.command);

    Ok(InstallToolResponse::Success {
        name: params.name,
        installed_version,
        duration_ms: start.elapsed().as_millis() as u64,
        hook_executed: spec.has_default_hook(),
    })
}
```

#### 1.3 Safety Module

```rust
// src/mcp/safety.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    check_times: Mutex<Vec<Instant>>,
    install_times: Mutex<Vec<Instant>>,
    max_checks_per_minute: usize,
    max_installs_per_minute: usize,
}

impl RateLimiter {
    pub fn check_install_limit(&self) -> Result<(), McpError> {
        let mut times = self.install_times.lock().unwrap();
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Remove old entries
        times.retain(|t| *t > one_minute_ago);

        if times.len() >= self.max_installs_per_minute {
            return Err(McpError::rate_limited(
                "Install rate limit exceeded. Please wait before installing more tools."
            ));
        }

        times.push(now);
        Ok(())
    }
}

pub fn check_allowlist(tool: &str, config: &McpConfig) -> Result<(), McpError> {
    if let Some(ref allowlist) = config.allowlist {
        if !allowlist.iter().any(|t| t.eq_ignore_ascii_case(tool)) {
            return Err(McpError::not_allowed(
                format!("Tool '{}' is not in the MCP allowlist", tool)
            ));
        }
    }
    Ok(())
}

pub fn check_denylist(tool: &str, config: &McpConfig) -> Result<(), McpError> {
    if let Some(ref denylist) = config.denylist {
        if denylist.iter().any(|t| t.eq_ignore_ascii_case(tool)) {
            return Err(McpError::denied(
                format!("Tool '{}' is in the MCP denylist and cannot be installed", tool)
            ));
        }
    }
    Ok(())
}
```

### Phase 2: CLI Integration (Week 1-2)

#### 2.1 MCP Subcommand

```rust
// src/main.rs additions
#[derive(Parser)]
enum Commands {
    // ... existing commands ...

    /// Run Jarvy as an MCP server
    #[command(name = "mcp")]
    Mcp(McpArgs),
}

#[derive(Args)]
struct McpArgs {
    /// Transport mode
    #[arg(long, default_value = "stdio")]
    transport: Transport,

    /// Config file path
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(ValueEnum, Clone)]
enum Transport {
    Stdio,
    // Sse, // Future: Server-Sent Events for web clients
}
```

#### 2.2 Entry Point

```rust
// src/mcp/mod.rs
pub async fn run_mcp_server(args: McpArgs) -> Result<()> {
    let config = McpConfig::load(args.config)?;

    let server = JarvyMcpServer::new(config);

    match args.transport {
        Transport::Stdio => server.run_stdio().await,
    }
}
```

### Phase 3: Distribution (Week 2-3)

#### 3.1 npm Package

```json
// package.json
{
  "name": "@jarvy/mcp-server",
  "version": "1.0.0",
  "description": "MCP server for safe development tool installation via Jarvy",
  "bin": {
    "jarvy-mcp": "./bin/jarvy-mcp"
  },
  "scripts": {
    "postinstall": "node scripts/download-binary.js"
  },
  "os": ["darwin", "linux", "win32"],
  "cpu": ["x64", "arm64"],
  "keywords": ["mcp", "model-context-protocol", "jarvy", "devtools", "claude"],
  "repository": "https://github.com/jarvy-dev/jarvy",
  "license": "MIT"
}
```

```javascript
// scripts/download-binary.js
const { platform, arch } = process;
const version = require('../package.json').version;

const PLATFORMS = {
  darwin: { x64: 'x86_64-apple-darwin', arm64: 'aarch64-apple-darwin' },
  linux: { x64: 'x86_64-unknown-linux-gnu', arm64: 'aarch64-unknown-linux-gnu' },
  win32: { x64: 'x86_64-pc-windows-msvc' }
};

const target = PLATFORMS[platform]?.[arch];
if (!target) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  process.exit(1);
}

const url = `https://github.com/jarvy-dev/jarvy/releases/download/v${version}/jarvy-v${version}-${target}.tar.gz`;
// Download and extract...
```

#### 3.2 Docker Image

```dockerfile
# Dockerfile.mcp
FROM rust:1.75-alpine AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin jarvy

FROM alpine:3.19
COPY --from=builder /build/target/release/jarvy /usr/local/bin/
ENTRYPOINT ["jarvy", "mcp"]
```

```yaml
# docker-compose.mcp.yml (for testing)
version: '3.8'
services:
  jarvy-mcp:
    build:
      context: .
      dockerfile: Dockerfile.mcp
    stdin_open: true
    tty: true
```

#### 3.3 MCP Registry Manifests

```json
// smithery.json (for smithery.ai registry)
{
  "name": "jarvy",
  "title": "Jarvy Tool Installer",
  "description": "Safe cross-platform development tool installation for LLMs",
  "version": "1.0.0",
  "author": "Jarvy Team",
  "license": "MIT",
  "repository": "https://github.com/jarvy-dev/jarvy",
  "install": {
    "npm": "@jarvy/mcp-server",
    "docker": "jarvy/mcp-server"
  },
  "capabilities": {
    "tools": true,
    "resources": true,
    "prompts": true
  },
  "tools": [
    {
      "name": "jarvy_list_tools",
      "description": "List available development tools"
    },
    {
      "name": "jarvy_check_tool",
      "description": "Check if a tool is installed"
    },
    {
      "name": "jarvy_install_tool",
      "description": "Install a development tool"
    }
  ]
}
```

### Phase 4: Client Configurations (Week 3)

#### 4.1 Claude Desktop Configuration

```json
// ~/Library/Application Support/Claude/claude_desktop_config.json (macOS)
// %APPDATA%\Claude\claude_desktop_config.json (Windows)
{
  "mcpServers": {
    "jarvy": {
      "command": "npx",
      "args": ["-y", "@jarvy/mcp-server"],
      "env": {}
    }
  }
}
```

Alternative with direct binary:

```json
{
  "mcpServers": {
    "jarvy": {
      "command": "jarvy",
      "args": ["mcp"],
      "env": {}
    }
  }
}
```

#### 4.2 Cursor Configuration

```json
// ~/.cursor/mcp.json
{
  "servers": {
    "jarvy": {
      "command": "jarvy",
      "args": ["mcp"]
    }
  }
}
```

#### 4.3 VS Code + Continue

```json
// .continue/config.json
{
  "mcpServers": [
    {
      "name": "jarvy",
      "command": "jarvy",
      "args": ["mcp"]
    }
  ]
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_tools_returns_all() {
        let params = ListToolsParams::default();
        let response = handle_list_tools(params).await.unwrap();
        assert!(response.count > 50); // We have 80+ tools
    }

    #[tokio::test]
    async fn test_check_tool_unknown() {
        let params = CheckToolParams { name: "nonexistent_xyz".to_string() };
        let result = handle_check_tool(params).await;
        assert!(matches!(result, Err(McpError::UnknownTool(_))));
    }

    #[tokio::test]
    async fn test_install_dry_run_default() {
        let params = InstallToolParams {
            name: "jq".to_string(),
            version: None,
            dry_run: None, // Should default to true
        };
        let config = McpConfig::default();
        let rate_limiter = RateLimiter::new(&config);

        let response = handle_install_tool(params, &config, &rate_limiter).await.unwrap();
        assert!(matches!(response, InstallToolResponse::DryRun { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess() {
        let config = McpConfig { max_installs_per_minute: 2, ..Default::default() };
        let limiter = RateLimiter::new(&config);

        assert!(limiter.check_install_limit().is_ok());
        assert!(limiter.check_install_limit().is_ok());
        assert!(limiter.check_install_limit().is_err()); // 3rd should fail
    }

    #[tokio::test]
    async fn test_denylist_blocks_tool() {
        let config = McpConfig {
            denylist: Some(vec!["brew".to_string()]),
            ..Default::default()
        };

        let result = check_denylist("brew", &config);
        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
// tests/mcp_integration.rs
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};

#[test]
fn test_mcp_initialize() {
    let mut child = Command::new("cargo")
        .args(["run", "--", "mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let stdin = child.stdin.as_mut().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());

    // Send initialize request
    let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    writeln!(stdin, "{}", init_request).unwrap();

    // Read response
    let mut response = String::new();
    stdout.lines().next().unwrap().unwrap();

    assert!(response.contains("jarvy"));

    child.kill().unwrap();
}
```

### E2E Tests with MCP Inspector

```bash
# Test with MCP Inspector CLI
npx @anthropic-ai/mcp-inspector jarvy mcp

# Interactive testing
> tools/list
> tools/call jarvy_check_tool {"name": "git"}
> tools/call jarvy_install_tool {"name": "ripgrep", "dry_run": true}
```

## Success Metrics

| Metric | Target |
|--------|--------|
| npm weekly downloads | 1,000+ (after 3 months) |
| GitHub stars (MCP contribution) | 100+ |
| Tool check latency (p99) | < 200ms |
| Install dry-run latency (p99) | < 50ms |
| Claude Desktop integration success rate | > 99% |
| Zero security incidents | 0 |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LLM bypasses confirmation | Low | High | Confirmation via stderr (not MCP response), audit logging |
| Rate limit circumvention | Low | Medium | Per-process limits, not just per-request |
| npm supply chain attack | Low | High | Signed releases, verify checksums in postinstall |
| MCP protocol changes | Medium | Medium | Pin to stable protocol version, monitor Anthropic releases |
| Platform detection failures | Low | Low | Fallback to manual platform specification |

## Dependencies

### New Crates

```toml
# Cargo.toml additions
[dependencies]
# MCP protocol support
jsonrpc-core = "18.0"       # JSON-RPC implementation
async-std = "1.12"          # Async runtime for stdio

# Rate limiting
governor = "0.6"            # Token bucket rate limiter

# Serialization (already have serde)
serde_json = "1.0"          # JSON for MCP messages
```

### Build Dependencies

```toml
[build-dependencies]
# For npm package generation
npm-pkg = "0.1"             # Optional: generate package.json from Cargo.toml
```

## Files to Create

```
src/mcp/
├── mod.rs              # Module exports
├── server.rs           # MCP server main loop
├── transport.rs        # Stdio transport
├── tools.rs            # Tool handler implementations
├── resources.rs        # Resource handler implementations
├── prompts.rs          # Prompt handler implementations
├── safety.rs           # Rate limiting, allowlist/denylist
├── config.rs           # MCP configuration loading
├── audit.rs            # Audit logging
└── error.rs            # MCP error types

dist/mcp/
├── package.json        # npm package manifest
├── scripts/
│   └── download-binary.js
├── bin/
│   └── jarvy-mcp       # Shell wrapper for npm
└── README.md           # npm package README

dist/docker/
├── Dockerfile.mcp      # MCP server Docker image
└── docker-compose.mcp.yml

docs/mcp/
├── README.md           # MCP integration guide
├── claude-desktop.md   # Claude Desktop setup
├── cursor.md           # Cursor setup
└── safety.md           # Security documentation
```

## Publishing Checklist

### npm Registry
- [ ] Create npm organization `@jarvy`
- [ ] Configure npm publish in CI
- [ ] Add 2FA requirement for publishing
- [ ] Create npmjs.com package page with docs

### Docker Hub
- [ ] Create Docker Hub organization `jarvy`
- [ ] Configure Docker publish in CI
- [ ] Add vulnerability scanning

### MCP Registries
- [ ] Submit to smithery.ai
- [ ] Submit to mcp.run
- [ ] Add to awesome-mcp-servers list

### Documentation
- [ ] Update main README with MCP section
- [ ] Create MCP-specific documentation
- [ ] Add video tutorial for Claude Desktop setup

## Future Enhancements (v2+)

1. **SSE Transport**: Support Server-Sent Events for web-based MCP clients
2. **Tool bundles**: Install predefined sets of tools (e.g., "web-dev", "data-science")
3. **Version negotiation**: Allow specifying tool versions in check/install
4. **Progress streaming**: Stream installation progress via MCP notifications
5. **Tool recommendations**: Suggest tools based on project files (detect package.json → suggest node)
6. **Configuration sync**: Sync MCP config across machines
