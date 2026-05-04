---
title: "Tool Dependencies - Jarvy"
description: "How Jarvy orders tool installs. Strict and flexible dependencies, topological sort, and what happens when deps are missing."
---

# Tool Dependencies

Tools depend on other tools. `kubectl` needs a Kubernetes cluster. `lazydocker` needs Docker. Jarvy models this with two dependency kinds and orders installs automatically via topological sort.

## Two Kinds of Dependencies

### Strict (`depends_on`)

ALL listed tools must be available for the dependent tool to function.

```rust
define_tool!(LAZYDOCKER, {
    command: "lazydocker",
    macos: { brew: "lazydocker" },
    depends_on: &["docker"],
});
```

Lazydocker is a TUI for the Docker daemon — without Docker installed, it has nothing to talk to.

### Flexible (`depends_on_one_of`)

AT LEAST ONE of the listed tools must be available. Order is preference.

```rust
define_tool!(KUBECTL, {
    command: "kubectl",
    macos: { brew: "kubectl" },
    depends_on_one_of: &["minikube", "kind", "k3d", "docker", "podman"],
});
```

`kubectl` works against any Kubernetes cluster — local dev clusters (minikube/kind/k3d) or container runtimes that ship with Kubernetes (Docker Desktop, Podman). Jarvy doesn't care which, just that one is there.

## Dependency Resolution

When you run `jarvy setup` with several tools, Jarvy:

1. Loads each tool's declared deps
2. Builds a dependency graph
3. Topologically sorts installs so deps precede dependents
4. For each tool, classifies the dep status:

| Status | Meaning |
|--------|---------|
| `Satisfied` | All required deps already installed |
| `WillInstallStrict` | Strict deps in the install list, will install first |
| `WillInstallFlexible` | At least one flexible option in the install list, will install first |
| `MissingRequired` | Strict dep missing — warn, install anyway, may not work |
| `MissingFlexible` | No flexible option installed or in list — advisory warning |

## Examples

### Example 1: kubectl alone

```toml
[provisioner]
kubectl = "latest"
```

No cluster provider in the install list. Jarvy:
- Installs `kubectl`
- Warns: "kubectl needs a Kubernetes cluster (minikube, kind, k3d, docker, or podman). None detected."

### Example 2: kubectl + docker

```toml
[provisioner]
kubectl = "latest"
docker = "latest"
```

Order: `docker` → `kubectl`. The flexible dep is satisfied by `docker` in the list.

### Example 3: kubectl + minikube + docker

```toml
[provisioner]
kubectl = "latest"
minikube = "latest"
docker = "latest"
```

Order: `docker` → `minikube` → `kubectl`.

- `minikube` has flexible dep on `[docker, podman]` → satisfied by docker
- `kubectl` has flexible dep on `[minikube, kind, k3d, docker, podman]` → satisfied by minikube (first match)

### Example 4: lazydocker without docker

```toml
[provisioner]
lazydocker = "latest"
```

Strict dep `docker` is missing and not in the install list. Jarvy:
- Warns: "lazydocker requires docker, which is not installed and not in your config."
- Installs lazydocker anyway (it's the user's choice)
- The tool will fail at runtime

To suppress dep warnings: `jarvy setup --ignore-missing-deps`.

## Inspecting Dependencies

```bash
# Show a single tool's deps
jarvy explain kubectl
# → Lists strict + flexible deps, supported platforms

# Show install order without running it
jarvy setup --dry-run
# → Prints the topologically sorted install list

# Show all tools with their deps
jarvy tools --index --format json | jq '.tools[] | {name, depends_on, depends_on_one_of}'
```

## Common Dependency Patterns

| Tool | Strict | Flexible | Reason |
|------|--------|----------|--------|
| `lazydocker` | docker | – | Docker TUI |
| `kind` | docker | – | Kubernetes-in-Docker |
| `kubectl` | – | minikube, kind, k3d, docker, podman | Needs any K8s cluster |
| `helm` | – | kubectl | K8s package manager |
| `k9s` | – | kubectl | K8s TUI |
| `minikube` | – | docker, podman | Local cluster runtime |
| `dive` | – | docker, podman | Image-layer explorer |
| `terraform` | – | – | Standalone |
| `ansible` | python | – | Python-based |

## For Tool Authors

When writing a new tool with `define_tool!`, add deps if the tool genuinely cannot function without them:

```rust
define_tool!(MYTOOL, {
    command: "mytool",
    macos: { brew: "mytool" },
    linux: { uniform: "mytool" },
    windows: { winget: "Pub.Mytool" },

    // Strict: tool literally won't run without these
    depends_on: &["docker"],

    // Flexible: tool needs ONE of these to be useful
    depends_on_one_of: &["kubectl", "oc"],
});
```

Don't over-declare. If `mytool` "works better" with another tool but doesn't strictly need it, leave it out — let users discover the combo on their own. Aim for "would crash or be useless without it."

## API for Programmatic Use

In code (or via the MCP server's `jarvy_get_tool`):

```rust
// src/tools/spec.rs
pub fn get_tool_dependencies(name: &str) -> Vec<&'static str>;
pub fn get_tool_flexible_dependencies(name: &str) -> Vec<&'static str>;
pub fn check_tool_dependencies(name: &str, install_list: &[&str]) -> DependencyCheckResult;
pub fn order_tools_by_dependencies(tools: &[&str]) -> Vec<&str>;
```

## Module

- Source: `src/tools/spec.rs`
- Macro: `define_tool!` in `src/tools/spec.rs`
- See also: [Adding Tools](adding-tools.md), [MCP Server](mcp-server.md#tool-dependencies)
