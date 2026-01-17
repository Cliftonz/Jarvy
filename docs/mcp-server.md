# Jarvy MCP Server

Jarvy exposes a Model Context Protocol (MCP) server that enables LLMs like Claude, GPT, and other AI assistants to safely discover, verify, and install development tools.

## Quick Start

```bash
# Start the MCP server
jarvy mcp

# With custom configuration
jarvy mcp --config ~/.jarvy/custom-mcp.toml
```

## Integration with AI Clients

### Claude Desktop

Add to your Claude Desktop configuration file:

**macOS**: `~/.config/claude-desktop/claude-desktop.json`
**Windows**: `%APPDATA%\Claude\claude-desktop.json`

```json
{
  "mcpServers": {
    "jarvy": {
      "command": "jarvy",
      "args": ["mcp"]
    }
  }
}
```

### Cursor

Add to your Cursor settings (`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "jarvy": {
      "command": "jarvy",
      "args": ["mcp"]
    }
  }
}
```

### VS Code with Continue

Add to your Continue configuration:

```json
{
  "models": [...],
  "mcpServers": {
    "jarvy": {
      "command": "jarvy",
      "args": ["mcp"]
    }
  }
}
```

## Available MCP Tools

The MCP server exposes 5 tools for tool management:

### jarvy_list_tools

List all tools Jarvy can install with optional filtering.

**Parameters:**
- `category` (optional): Filter by category - `language`, `database`, `container`, `cli`, `editor`, `all`
- `platform` (optional): Filter by platform - `macos`, `linux`, `windows`, `current`
- `search` (optional): Search tools by name

**Example:**
```json
{
  "name": "jarvy_list_tools",
  "arguments": {
    "search": "docker",
    "platform": "current"
  }
}
```

### jarvy_get_tool

Get detailed information about a specific tool.

**Parameters:**
- `name` (required): Tool name (e.g., `git`, `docker`, `node`)

**Example:**
```json
{
  "name": "jarvy_get_tool",
  "arguments": {
    "name": "ripgrep"
  }
}
```

### jarvy_check_tool

Check if a tool is installed and get its version.

**Parameters:**
- `name` (required): Tool name to check

**Example:**
```json
{
  "name": "jarvy_check_tool",
  "arguments": {
    "name": "git"
  }
}
```

**Response:**
```json
{
  "name": "git",
  "installed": true,
  "version": "2.43.0",
  "path": "/usr/bin/git"
}
```

### jarvy_check_multiple

Check installation status of multiple tools at once.

**Parameters:**
- `tools` (required): Array of tool names

**Example:**
```json
{
  "name": "jarvy_check_multiple",
  "arguments": {
    "tools": ["git", "docker", "node", "rust"]
  }
}
```

### jarvy_install_tool

Install a development tool (requires user confirmation).

**Parameters:**
- `name` (required): Tool name to install
- `version` (optional): Version hint (default: `latest`)
- `dry_run` (optional): Preview without executing (default: `true`)

**Example:**
```json
{
  "name": "jarvy_install_tool",
  "arguments": {
    "name": "ripgrep",
    "dry_run": true
  }
}
```

**Safety Note:** By default, `dry_run` is `true` to prevent accidental installations. Set `dry_run: false` to actually install, which will prompt for user confirmation.

## Available MCP Resources

### jarvy://tools/index

Complete tool index as JSON with all supported tools.

### jarvy://platform/info

Current platform information including OS, architecture, and available package managers.

### jarvy://tools/{name}

Detailed information for a specific tool (e.g., `jarvy://tools/docker`).

## Available MCP Prompts

### setup_dev_environment

Guided prompt for setting up a development environment.

**Arguments:**
- `project_type`: One of `rust`, `node`, `python`, `go`, `java`, `ruby`, `devops`, `data-science`

### diagnose_missing_tools

Diagnostic prompt that checks common development tools and suggests installations.

## Configuration

Create `~/.jarvy/mcp-config.toml` to customize MCP server behavior:

```toml
[mcp]
# Require confirmation before installing (default: true)
require_confirmation = true

# Default to dry-run mode (default: true)
default_dry_run = true

[rate_limits]
# Maximum tool checks per minute (default: 10)
checks_per_minute = 10

# Maximum installations per minute (default: 3)
installs_per_minute = 3

[allowlist]
# Only allow these tools to be installed (optional)
# If set, only listed tools can be installed
tools = ["git", "node", "python", "rust", "docker"]

# [denylist]
# Block specific tools (takes precedence over allowlist)
# tools = ["dangerous-tool"]

[audit]
# Enable audit logging (default: true)
enabled = true

# Custom log path (default: ~/.jarvy/mcp-audit.log)
# log_path = "~/.jarvy/mcp-audit.log"
```

## Safety Features

### Dry Run by Default

All installation requests default to `dry_run: true`, showing what would happen without executing. This prevents LLMs from accidentally installing software.

### Rate Limiting

The server implements sliding window rate limiting:
- **10 checks per minute** for `check_tool` and `check_multiple`
- **3 installs per minute** for `install_tool`

### Allowlist/Denylist

Control which tools can be installed:
- **Allowlist**: Only specified tools can be installed
- **Denylist**: Specified tools are blocked (takes precedence)

### User Confirmation

When `require_confirmation` is enabled (default), actual installations prompt for user confirmation via stderr (not through MCP responses).

### Audit Logging

All MCP operations are logged to `~/.jarvy/mcp-audit.log` in JSON Lines format:

```json
{"timestamp":"2026-01-16T12:36:40Z","action":"install_tool","client":"claude-desktop","tool":"ripgrep","success":true,"version":"14.1.0"}
```

## Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -32001 | Tool execution failed |
| -32002 | Tool not found |
| -32003 | Installation denied (denylist) |
| -32004 | Rate limited |
| -32005 | User declined |
| -32006 | Sudo required |
| -32007 | Timeout |

## Troubleshooting

### Server not starting

1. Ensure Jarvy is installed and in your PATH:
   ```bash
   which jarvy
   jarvy --version
   ```

2. Test the MCP server manually:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | jarvy mcp
   ```

### Rate limit errors

If you see rate limit errors, wait 60 seconds for the sliding window to reset, or adjust limits in `~/.jarvy/mcp-config.toml`.

### Tools not installing

1. Check if the tool is in the denylist
2. Ensure `dry_run` is set to `false`
3. Verify user confirmation was accepted
4. Check audit log for error details: `tail -f ~/.jarvy/mcp-audit.log`

### Permission errors

Some tools require elevated permissions. The MCP server will return error code `-32006` (Sudo required) when this occurs.

## Protocol Version

This implementation targets MCP protocol version `2024-11-05`.
