//! Benchmarks for tool registry lookup operations
//!
//! Measures the performance of looking up tools in the global registry.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;

/// Simulate registry lookup with a HashMap (similar to the actual registry)
fn create_mock_registry() -> HashMap<String, &'static str> {
    let mut registry = HashMap::new();

    // Add common tools
    let tools = [
        "git",
        "node",
        "npm",
        "yarn",
        "pnpm",
        "bun",
        "rust",
        "cargo",
        "rustup",
        "go",
        "python",
        "pip",
        "docker",
        "kubectl",
        "helm",
        "terraform",
        "ansible",
        "jq",
        "ripgrep",
        "fd",
        "bat",
        "eza",
        "fzf",
        "zoxide",
        "starship",
        "neovim",
        "vim",
        "tmux",
        "lazygit",
        "gh",
        "aws-cli",
        "gcloud",
        "azure-cli",
        "redis",
        "postgresql",
        "mongodb",
        "gradle",
        "maven",
        "java",
        "kotlin",
        "scala",
        "ruby",
        "rails",
        "php",
        "composer",
        "nginx",
        "caddy",
    ];

    for tool in tools {
        registry.insert(tool.to_string(), tool);
    }

    registry
}

fn bench_registry_lookup(c: &mut Criterion) {
    let registry = create_mock_registry();

    let mut group = c.benchmark_group("registry_lookup");

    // Lookup existing tools
    let existing_tools = ["git", "node", "docker", "terraform", "starship"];
    for tool in existing_tools {
        group.bench_with_input(BenchmarkId::new("existing", tool), tool, |b, tool| {
            b.iter(|| registry.get(black_box(tool)));
        });
    }

    // Lookup non-existing tools
    let missing_tools = ["notool", "fakecli", "unknown123"];
    for tool in missing_tools {
        group.bench_with_input(BenchmarkId::new("missing", tool), tool, |b, tool| {
            b.iter(|| registry.get(black_box(tool)));
        });
    }

    group.finish();
}

fn bench_registry_iteration(c: &mut Criterion) {
    let registry = create_mock_registry();

    let mut group = c.benchmark_group("registry_iteration");

    group.bench_function("iter_all_keys", |b| {
        b.iter(|| {
            for key in registry.keys() {
                black_box(key);
            }
        });
    });

    group.bench_function("iter_all_values", |b| {
        b.iter(|| {
            for value in registry.values() {
                black_box(value);
            }
        });
    });

    group.bench_function("count_tools", |b| {
        b.iter(|| black_box(registry.len()));
    });

    group.finish();
}

fn bench_bulk_lookup(c: &mut Criterion) {
    let registry = create_mock_registry();
    let tools_to_lookup: Vec<&str> = vec![
        "git",
        "node",
        "docker",
        "kubectl",
        "terraform",
        "jq",
        "ripgrep",
        "starship",
        "gh",
        "neovim",
    ];

    let mut group = c.benchmark_group("bulk_lookup");

    group.bench_function("lookup_10_tools", |b| {
        b.iter(|| {
            for tool in &tools_to_lookup {
                black_box(registry.get(*tool));
            }
        });
    });

    group.bench_function("collect_10_tools", |b| {
        b.iter(|| {
            let results: Vec<_> = tools_to_lookup
                .iter()
                .filter_map(|t| registry.get(*t))
                .collect();
            black_box(results);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_registry_lookup,
    bench_registry_iteration,
    bench_bulk_lookup
);
criterion_main!(benches);
