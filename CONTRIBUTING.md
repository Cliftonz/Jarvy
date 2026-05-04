# Contributing to Jarvy

Thank you for your interest in contributing to Jarvy! This document provides guidelines and information for contributors.

## Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/bearbinary/jarvy.git
   cd jarvy
   ```

2. Install Rust (1.85+):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Build the project:
   ```bash
   cargo build
   ```

## Testing

### Running Tests

```bash
# Run all tests
cargo test --verbose -- --show-output

# Run a specific test
cargo test --test cli_dispatch -- --show-output

# Run tests with test mode (disables interactive prompts)
JARVY_TEST_MODE=1 cargo test
```

### Test Environment Variables

- `JARVY_TEST_MODE=1` - Disables interactive prompts
- `JARVY_FAST_TEST=1` - Skips external command execution

### Code Coverage

We use `cargo-llvm-cov` for code coverage measurement.

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --all-features --workspace --html

# View report
open target/llvm-cov/html/index.html
```

**Coverage Target:** 80%+ line coverage

### Property-Based Testing

We use `proptest` for property-based testing. Property tests are located in `tests/property/`.

```bash
# Run property tests
cargo test --test property

# Run with more test cases
PROPTEST_CASES=10000 cargo test --test property
```

**Guidelines:**
- Each property test should run at least 1000 cases
- Document the properties being tested
- Use shrinking to find minimal failing inputs

### Fuzz Testing

We use `cargo-fuzz` with libfuzzer for fuzz testing. Requires nightly Rust.

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run a fuzz target
cd fuzz
cargo +nightly fuzz run fuzz_config_parser

# Run for specific duration (seconds)
cargo +nightly fuzz run fuzz_config_parser -- -max_total_time=300
```

**Fuzz Targets:**
- `fuzz_config_parser` - Tests TOML parsing
- `fuzz_version_parser` - Tests semver parsing
- `fuzz_toml_input` - Tests structured config input
- `fuzz_tool_spec` - Tests tool specification parsing

### Benchmarks

We use `criterion` for benchmarking.

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench config_parsing

# Generate HTML reports
cargo bench -- --noplot
open target/criterion/report/index.html
```

**Benchmarks:**
- `config_parsing` - Config file parsing performance
- `registry_lookup` - Tool registry lookup performance
- `version_comparison` - Version comparison operations

### Mutation Testing

We use `cargo-mutants` to verify test effectiveness.

```bash
# Install cargo-mutants
cargo install cargo-mutants

# Run mutation testing on critical modules
cargo mutants -f src/config.rs -f src/tools/common.rs -- --lib

# View results
cat mutants.out/missed.txt
```

**Mutation Score Target:** 60%+

### Integration Testing with Containers

Test containers are provided for Ubuntu, Fedora, and Alpine:

```bash
# Build and run Ubuntu tests
docker build -f tests/docker/ubuntu.dockerfile -t jarvy-test-ubuntu .
docker run --rm jarvy-test-ubuntu

# Build and run Fedora tests
docker build -f tests/docker/fedora.dockerfile -t jarvy-test-fedora .
docker run --rm jarvy-test-fedora

# Build and run Alpine tests
docker build -f tests/docker/alpine.dockerfile -t jarvy-test-alpine .
docker run --rm jarvy-test-alpine
```

## Code Quality

### Formatting

```bash
cargo fmt --all
```

### Linting

```bash
cargo clippy --all-features -- -D warnings
```

### Pre-commit Checklist

Before submitting a PR:

1. [ ] `cargo fmt --all`
2. [ ] `cargo clippy --all-features -- -D warnings`
3. [ ] `cargo test --verbose`
4. [ ] `cargo build --release`

## Adding a New Tool

Use the scaffolding tool:

```bash
cargo run -p cargo-jarvy -- new-tool <tool-name>
```

This creates the necessary files in `src/tools/<tool-name>/`.

### Tool Implementation Pattern

Each tool lives in `src/tools/<name>/` with two files:
- `mod.rs` -- re-exports with `pub use <name>::*;`
- `<name>.rs` -- tool definition using the `define_tool!` macro

```rust
//! tool-name - Brief description
use crate::define_tool;

define_tool!(TOOL_NAME, {
    command: "tool-name",
    macos: { brew: "tool-name" },
    linux: { uniform: "tool-name" },
    windows: { winget: "Publisher.ToolName" },
    // Optional fields:
    // bsd: { pkg: "tool-name" },
    // custom_install: Some(custom_install_fn),
    // default_hook: { description: "Configure tool", script: "echo setup" },
    // depends_on: &["docker"],
    // depends_on_one_of: &["minikube", "kind", "docker"],
    // category: "devops",
});
```

See `src/tools/spec.rs` for the full `ToolSpec` struct and macro documentation.

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `chore:` - Maintenance tasks
- `refactor:` - Code refactoring
- `test:` - Test changes

Example:
```
feat: add support for tool-name installation

- Add tool definition using define_tool! macro
- Support macOS via Homebrew
- Support Linux via apt/dnf/pacman
```

## CI Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| test.yml | Push/PR | Run tests |
| clippy.yml | Push/PR | Lint code |
| coverage.yml | Push/PR | Generate coverage report |
| benchmark.yml | Push/PR | Run benchmarks |
| fuzz.yml | Push/PR/Weekly | Fuzz testing |
| mutation.yml | Weekly | Mutation testing |

## Questions?

Open an issue or discussion on GitHub!
