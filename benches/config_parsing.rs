//! Benchmarks for jarvy.toml config file parsing
//!
//! Measures parsing performance for small, medium, and large config files.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Small config - basic tools only
const SMALL_CONFIG: &str = r#"
[provisioner]
git = "latest"
node = "20"
"#;

/// Medium config - typical project with hooks and env
const MEDIUM_CONFIG: &str = r#"
[privileges]
use_sudo = true

[privileges.per_os]
linux = true
macos = false

[provisioner]
git = "latest"
node = "20"
rust = "1.75"
docker = "latest"
kubectl = "latest"
terraform = "1.5"
jq = "latest"
ripgrep = "latest"

[hooks]
pre_setup = "echo 'Starting setup...'"
post_setup = "echo 'Setup complete!'"

[hooks.config]
shell = "zsh"
timeout = 300

[hooks.node]
post_install = "npm install -g yarn typescript"

[hooks.rust]
post_install = "rustup component add clippy rustfmt"

[env.vars]
PROJECT_ROOT = "$PWD"
NODE_ENV = "development"
RUST_BACKTRACE = "1"

[env.config]
generate_dotenv = true
dotenv_path = ".env"
"#;

/// Large config - enterprise with many tools
const LARGE_CONFIG: &str = r#"
[privileges]
use_sudo = true

[privileges.per_os]
linux = true
macos = false
windows = false

[provisioner]
git = "latest"
node = { version = "20", version_manager = true }
rust = "1.75"
go = "1.21"
python = "3.12"
docker = "latest"
kubectl = "latest"
helm = "latest"
terraform = "1.5"
ansible = "latest"
jq = "latest"
ripgrep = "latest"
fd = "latest"
bat = "latest"
eza = "latest"
fzf = "latest"
zoxide = "latest"
starship = "latest"
neovim = "latest"
tmux = "latest"
lazygit = "latest"
gh = "latest"
aws-cli = "latest"
gcloud = "latest"
azure-cli = "latest"
redis = "latest"
postgresql = "latest"
mongodb = "latest"
gradle = "latest"
maven = "latest"

[hooks]
pre_setup = """
echo 'Starting enterprise setup...'
echo 'This may take a while...'
"""
post_setup = """
echo 'Setup complete!'
echo 'Please restart your shell for all changes to take effect.'
"""

[hooks.config]
shell = "zsh"
timeout = 600
continue_on_error = false

[hooks.node]
post_install = "npm install -g yarn typescript eslint prettier"

[hooks.rust]
post_install = "rustup component add clippy rustfmt llvm-tools-preview"

[hooks.python]
post_install = "pip install poetry black ruff mypy"

[hooks.go]
post_install = "go install golang.org/x/tools/gopls@latest"

[env.vars]
PROJECT_ROOT = "$PWD"
NODE_ENV = "development"
RUST_BACKTRACE = "1"
GO111MODULE = "on"
PYTHONDONTWRITEBYTECODE = "1"
DOCKER_BUILDKIT = "1"

[env.secrets]
AWS_ACCESS_KEY_ID = { env = "AWS_ACCESS_KEY_ID", required = false }
AWS_SECRET_ACCESS_KEY = { env = "AWS_SECRET_ACCESS_KEY", required = false }

[env.config]
generate_dotenv = true
dotenv_path = ".env"
update_rc = false
add_to_gitignore = true

[services]
enabled = true
auto_start = false
compose_file = "docker/docker-compose.yml"
"#;

fn bench_config_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing");

    // Benchmark TOML parsing directly (what we're measuring)
    for (name, config) in [
        ("small", SMALL_CONFIG),
        ("medium", MEDIUM_CONFIG),
        ("large", LARGE_CONFIG),
    ] {
        group.bench_with_input(BenchmarkId::new("toml_parse", name), config, |b, config| {
            b.iter(|| {
                let _: toml::Value = toml::from_str(black_box(config)).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_toml_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("toml_roundtrip");

    for (name, config) in [
        ("small", SMALL_CONFIG),
        ("medium", MEDIUM_CONFIG),
        ("large", LARGE_CONFIG),
    ] {
        group.bench_with_input(
            BenchmarkId::new("parse_serialize", name),
            config,
            |b, config| {
                b.iter(|| {
                    let value: toml::Value = toml::from_str(black_box(config)).unwrap();
                    let _serialized = toml::to_string(&value).unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_config_parsing, bench_toml_roundtrip);
criterion_main!(benches);
