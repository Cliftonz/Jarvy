# PRD-045: Dependency Graph Visualization

## Overview

Enable Jarvy to visualize tool dependencies, installation order, and relationships as an interactive graph, helping users understand their environment structure and troubleshoot dependency issues.

## Problem Statement

As projects grow, understanding tool relationships becomes challenging:

- Users don't understand why certain tools are installed
- Dependency chains are opaque (why does X require Y?)
- Installation order issues are hard to diagnose
- Circular or conflicting dependencies are not visible
- Team discussions about tooling lack visual aids

A visual dependency graph makes these relationships clear and aids troubleshooting.

## Evidence

- "Why is this tool being installed?" is a common question
- Dependency issues are hard to debug without visibility
- Documentation often includes hand-drawn dependency diagrams
- Teams struggle to explain toolchain complexity to new members
- Tool removal impact is unclear without dependency view

## Requirements

### Functional Requirements

1. **Graph generation**: Generate dependency graph from config
2. **Multiple formats**: Output as ASCII, DOT, SVG, HTML
3. **Filtering**: Filter by tool, category, or depth
4. **Highlighting**: Highlight paths, cycles, or specific tools
5. **Legend**: Include legend for node/edge types
6. **Interactive**: HTML output with pan/zoom/search

### Non-Functional Requirements

1. **Fast**: Generate graph in <1 second
2. **Readable**: Clear layout even for complex graphs
3. **Portable**: ASCII works in any terminal
4. **Exportable**: Formats suitable for documentation
5. **Accessible**: Color-blind friendly palette

## Non-Goals

- Real-time graph updates during installation
- Version conflict resolution (just visualization)
- Dependency graph of language packages (npm, pip, etc.)
- System-level dependency visualization (shared libraries)
- Integration with external graph tools

## Feature Specifications

### 1. CLI Commands

```bash
# Show dependency graph in terminal (ASCII)
jarvy graph

# Output:
# Tool Dependency Graph
# =====================
#
#  node ──────────────────────────────────────────────────┐
#    │                                                    │
#    └──> nvm (version manager)                           │
#                                                         │
#  rust ──────────────────────────────────────────────────┤
#    │                                                    │
#    └──> rustup (version manager)                        │
#          │                                              │
#          └──> cargo                                     │
#                │                                        │
#                ├──> cargo-watch                         │
#                └──> cargo-nextest                       │
#                                                         │
#  docker ────────────────────────────────────────────────┤
#    │                                                    │
#    ├──> docker-compose                                  │
#    └──> lazydocker                                      │
#                                                         │
#  kubectl ───────────────────────────────────────────────┘
#    │
#    ├──> helm
#    ├──> k9s
#    └──> [docker OR minikube OR kind]  (flexible dep)
#
# Legend: ──> requires, [...] one-of

# Export as DOT format (for Graphviz)
jarvy graph --format dot > deps.dot
dot -Tpng deps.dot -o deps.png

# Export as SVG
jarvy graph --format svg > deps.svg

# Export as interactive HTML
jarvy graph --format html > deps.html

# Filter by specific tool
jarvy graph --tool kubectl

# Show installation order
jarvy graph --order

# Highlight dependency path between tools
jarvy graph --path rust kubectl

# Show only depth N from roots
jarvy graph --depth 2

# Include version information
jarvy graph --versions

# JSON output for programmatic use
jarvy graph --format json
```

### 2. ASCII Graph Output

```
Tool Dependency Graph
=====================

Configured Tools: 12
Dependencies: 8
Installation Order: 15 steps

                    ┌─────────┐
                    │   git   │
                    └────┬────┘
                         │
           ┌─────────────┴─────────────┐
           │                           │
      ┌────▼────┐                 ┌────▼────┐
      │   gh    │                 │  delta  │
      └─────────┘                 └─────────┘

      ┌─────────┐
      │  rust   │
      └────┬────┘
           │
    ┌──────┴──────┐
    │             │
┌───▼───┐   ┌─────▼─────┐
│ cargo │   │  rustfmt  │
└───┬───┘   └───────────┘
    │
┌───▼───────────┐
│  cargo-watch  │
└───────────────┘

      ┌──────────┐
      │  docker  │◀─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐
      └────┬─────┘                          │
           │                           (flexible)
    ┌──────┴──────┐                         │
    │             │                         │
┌───▼────┐  ┌─────▼─────┐            ┌──────┴─────┐
│compose │  │lazydocker │            │  kubectl   │
└────────┘  └───────────┘            └────────────┘

Legend:
  ─▶  strict dependency (required)
  ─ ▶ flexible dependency (one of)
  [ ] version manager relationship
```

### 3. DOT Format Output

```dot
digraph JarvyDependencies {
    // Graph settings
    rankdir=TB;
    node [shape=box, style=rounded, fontname="Helvetica"];
    edge [fontname="Helvetica", fontsize=10];

    // Subgraph for language runtimes
    subgraph cluster_runtimes {
        label="Runtimes";
        style=dashed;
        node;
        rust;
        python;
    }

    // Subgraph for container tools
    subgraph cluster_containers {
        label="Container Tools";
        style=dashed;
        docker;
        kubectl;
    }

    // Dependencies
    rust -> cargo [label="provides"];
    cargo -> "cargo-watch" [label="extends"];
    cargo -> "cargo-nextest" [label="extends"];

    docker -> "docker-compose" [label="requires"];
    docker -> lazydocker [label="requires"];

    kubectl -> docker [style=dashed, label="flexible"];
    kubectl -> minikube [style=dashed, label="flexible"];
    kubectl -> kind [style=dashed, label="flexible"];

    // Installation order
    { rank=same; git; rust; docker; }
    { rank=same; cargo; kubectl; }
}
```

### 4. JSON Format Output

```json
{
  "metadata": {
    "generated_at": "2024-01-20T15:00:00Z",
    "tool_count": 12,
    "dependency_count": 8
  },
  "nodes": [
    {
      "id": "rust",
      "version": "1.75.0",
      "category": "runtime",
      "install_method": "rustup",
      "installed": true
    },
    {
      "id": "cargo",
      "version": "1.75.0",
      "category": "build",
      "provided_by": "rust",
      "installed": true
    },
    {
      "id": "kubectl",
      "version": "1.29.0",
      "category": "ops",
      "installed": true
    }
  ],
  "edges": [
    {
      "from": "rust",
      "to": "cargo",
      "type": "provides"
    },
    {
      "from": "cargo",
      "to": "cargo-watch",
      "type": "strict"
    },
    {
      "from": "kubectl",
      "to": ["docker", "minikube", "kind"],
      "type": "flexible"
    }
  ],
  "installation_order": [
    "git",
    "rust",
    "cargo",
    "docker",
    "kubectl",
    "cargo-watch"
  ]
}
```

### 5. Interactive HTML Output

```html
<!-- Self-contained HTML with embedded D3.js visualization -->
<!DOCTYPE html>
<html>
<head>
    <title>Jarvy Dependency Graph</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        .node { cursor: pointer; }
        .node circle { fill: #fff; stroke: #333; stroke-width: 2px; }
        .node text { font: 12px sans-serif; }
        .link { fill: none; stroke: #999; stroke-width: 1.5px; }
        .link.flexible { stroke-dasharray: 5,5; }
        .selected { stroke: #f00; stroke-width: 3px; }
    </style>
</head>
<body>
    <div id="controls">
        <input type="text" id="search" placeholder="Search tools...">
        <button onclick="resetView()">Reset</button>
    </div>
    <svg id="graph"></svg>
    <script>
        // D3.js force-directed graph
        const data = /* JSON data embedded */;
        // ... visualization code
    </script>
</body>
</html>
```

## Technical Approach

### Module Structure

```
src/
  graph/
    mod.rs           # Public API
    builder.rs       # Graph construction
    layout.rs        # Layout algorithms
    ascii.rs         # ASCII renderer
    dot.rs           # DOT format renderer
    svg.rs           # SVG renderer
    html.rs          # Interactive HTML renderer
    json.rs          # JSON serializer
    commands.rs      # CLI command handlers
```

### Graph Data Structures

```rust
// src/graph/builder.rs
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ToolNode {
    pub name: String,
    pub version: Option<String>,
    pub category: ToolCategory,
    pub install_method: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeType {
    /// Tool A requires Tool B
    Strict,
    /// Tool A requires one of [B, C, D]
    Flexible,
    /// Tool A provides Tool B (e.g., rustup provides rust)
    Provides,
    /// Tool A extends Tool B (e.g., cargo-watch extends cargo)
    Extends,
}

pub struct DependencyGraph {
    graph: DiGraph<ToolNode, EdgeType>,
    name_to_index: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    pub fn from_config(config: &JarvyConfig) -> Self {
        let mut graph = DiGraph::new();
        let mut name_to_index = HashMap::new();

        // Add nodes for all configured tools
        for (name, spec) in config.provisioner_iter() {
            let node = ToolNode {
                name: name.clone(),
                version: spec.version.clone(),
                category: get_tool_category(&name),
                install_method: spec.install_method.clone(),
                installed: tool_is_installed(&name),
            };
            let idx = graph.add_node(node);
            name_to_index.insert(name, idx);
        }

        // Add edges for dependencies
        for (name, _) in config.provisioner_iter() {
            if let Some(deps) = get_tool_dependencies(&name) {
                for dep in deps {
                    if let (Some(&from), Some(&to)) = (name_to_index.get(&name), name_to_index.get(&dep)) {
                        graph.add_edge(from, to, EdgeType::Strict);
                    }
                }
            }

            if let Some(flex_deps) = get_tool_flexible_dependencies(&name) {
                for dep in flex_deps {
                    if let (Some(&from), Some(&to)) = (name_to_index.get(&name), name_to_index.get(&dep)) {
                        graph.add_edge(from, to, EdgeType::Flexible);
                    }
                }
            }
        }

        Self { graph, name_to_index }
    }

    pub fn installation_order(&self) -> Vec<String> {
        // Topological sort
        use petgraph::algo::toposort;
        toposort(&self.graph, None)
            .map(|order| {
                order.iter()
                    .map(|idx| self.graph[*idx].name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn filter_by_tool(&self, tool: &str) -> Self {
        // Return subgraph containing tool and its dependencies
        // ...
    }

    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        use petgraph::algo::astar;
        // ...
    }

    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        use petgraph::algo::tarjan_scc;
        // ...
    }
}
```

### ASCII Renderer

```rust
// src/graph/ascii.rs

pub struct AsciiRenderer {
    width: usize,
    use_unicode: bool,
}

impl AsciiRenderer {
    pub fn render(&self, graph: &DependencyGraph) -> String {
        let mut output = String::new();

        output.push_str("Tool Dependency Graph\n");
        output.push_str("=====================\n\n");

        // Group by root nodes (tools with no dependencies)
        let roots = graph.roots();

        for root in roots {
            self.render_tree(&mut output, graph, &root, 0, &mut HashSet::new());
        }

        output.push_str("\nLegend:\n");
        output.push_str("  ──▶  strict dependency (required)\n");
        output.push_str("  ─ ▶  flexible dependency (one of)\n");

        output
    }

    fn render_tree(
        &self,
        output: &mut String,
        graph: &DependencyGraph,
        node: &str,
        depth: usize,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        let indent = "  ".repeat(depth);
        let connector = if depth > 0 {
            if self.use_unicode { "└──▶ " } else { "+--> " }
        } else {
            ""
        };

        output.push_str(&format!("{}{}{}\n", indent, connector, node));

        for (dep, edge_type) in graph.dependencies(node) {
            let edge_char = match edge_type {
                EdgeType::Strict => if self.use_unicode { "├──▶" } else { "|-->" },
                EdgeType::Flexible => if self.use_unicode { "├─ ▶" } else { "|..>" },
                EdgeType::Provides => if self.use_unicode { "├══▶" } else { "|==>" },
                EdgeType::Extends => if self.use_unicode { "├──▶" } else { "|-->" },
            };
            output.push_str(&format!("{}  {}{}\n", indent, edge_char, dep));
            self.render_tree(output, graph, &dep, depth + 1, visited);
        }
    }
}
```

### DOT Renderer

```rust
// src/graph/dot.rs

pub struct DotRenderer {
    include_versions: bool,
    cluster_by_category: bool,
}

impl DotRenderer {
    pub fn render(&self, graph: &DependencyGraph) -> String {
        let mut output = String::new();

        output.push_str("digraph JarvyDependencies {\n");
        output.push_str("    rankdir=TB;\n");
        output.push_str("    node [shape=box, style=rounded, fontname=\"Helvetica\"];\n");
        output.push_str("    edge [fontname=\"Helvetica\", fontsize=10];\n\n");

        // Group nodes by category
        if self.cluster_by_category {
            for category in &[ToolCategory::Runtime, ToolCategory::Build, ToolCategory::Dev, ToolCategory::Ops] {
                let tools: Vec<_> = graph.nodes_by_category(*category).collect();
                if !tools.is_empty() {
                    output.push_str(&format!("    subgraph cluster_{:?} {{\n", category));
                    output.push_str(&format!("        label=\"{:?}\";\n", category));
                    output.push_str("        style=dashed;\n");
                    for tool in tools {
                        output.push_str(&format!("        \"{}\";\n", tool));
                    }
                    output.push_str("    }\n\n");
                }
            }
        }

        // Add edges
        for (from, to, edge_type) in graph.edges() {
            let style = match edge_type {
                EdgeType::Strict => "",
                EdgeType::Flexible => " [style=dashed]",
                EdgeType::Provides => " [label=\"provides\"]",
                EdgeType::Extends => " [label=\"extends\"]",
            };
            output.push_str(&format!("    \"{}\" -> \"{}\"{};\n", from, to, style));
        }

        output.push_str("}\n");
        output
    }
}
```

## Implementation Steps

1. Create graph module structure
2. Add petgraph dependency
3. Implement DependencyGraph construction
4. Implement installation order calculation
5. Implement cycle detection
6. Implement ASCII renderer
7. Implement DOT renderer
8. Implement SVG renderer (from DOT)
9. Implement JSON serializer
10. Implement HTML renderer with D3.js
11. Implement filtering and path finding
12. Add CLI commands
13. Write tests for graph operations
14. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Time to understand dependencies | 10+ minutes | <1 minute |
| Dependency debugging sessions | Long, frustrating | Quick, visual |
| Documentation with diagrams | Manual | Auto-generated |
| Dependency cycle detection | Manual | Automatic |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Complex graphs hard to read | Medium | Medium | Filtering, clustering, multiple formats |
| Layout issues with many nodes | Low | Medium | Smart layout algorithms, zoom in HTML |
| External tool (Graphviz) required | Low | Low | ASCII default, SVG fallback |
| D3.js loading in HTML | Low | Low | Embed minimal D3 or use simpler viz |

## Dependencies

### New Dependencies
- `petgraph` - Graph data structure and algorithms

### Optional Dependencies
- Graphviz (external) - For DOT to PNG/PDF conversion

### Existing Dependencies
- `serde_json` - JSON output

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure | 0.25 days |
| Graph construction | 1 day |
| Installation order/cycles | 0.5 days |
| ASCII renderer | 1 day |
| DOT renderer | 0.5 days |
| SVG renderer | 0.5 days |
| JSON serializer | 0.25 days |
| HTML renderer | 1.5 days |
| Filtering/path finding | 0.5 days |
| CLI commands | 0.5 days |
| Testing | 1 day |
| Documentation | 0.5 days |
| **Total** | **8 days** |

## Files to Create/Modify

### New Files
- `src/graph/mod.rs`
- `src/graph/builder.rs`
- `src/graph/layout.rs`
- `src/graph/ascii.rs`
- `src/graph/dot.rs`
- `src/graph/svg.rs`
- `src/graph/html.rs`
- `src/graph/json.rs`
- `src/graph/commands.rs`
- `tests/graph_integration.rs`

### Modified Files
- `src/lib.rs` - Export graph module
- `src/main.rs` - Add graph subcommand
- `Cargo.toml` - Add petgraph dependency
- `CLAUDE.md` - Document graph command

---

*PRD-045 v1.0 | Dependency Graph Visualization | Priority: Low*
