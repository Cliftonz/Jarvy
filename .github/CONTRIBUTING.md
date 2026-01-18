# Contributing to Jarvy

Thank you for your interest in contributing to Jarvy! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust toolchain (stable, 1.75+)
- Git
- A package manager for your OS (Homebrew on macOS, apt/dnf on Linux, winget on Windows)

### Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/jarvy-dev/jarvy.git
cd jarvy

# Build in debug mode
cargo build

# Run tests
cargo test --verbose -- --show-output

# Run lints
cargo clippy --all-features -- -D warnings

# Format code
cargo fmt --all
```

## How to Contribute

### Reporting Bugs

1. Search [existing issues](https://github.com/jarvy-dev/jarvy/issues) to avoid duplicates
2. Use the [Bug Report template](https://github.com/jarvy-dev/jarvy/issues/new?template=bug_report.yml)
3. Include output from `jarvy doctor`
4. Provide minimal reproduction steps

### Suggesting Features

1. Check [existing feature requests](https://github.com/jarvy-dev/jarvy/issues?q=is%3Aissue+label%3Aenhancement)
2. Use the [Feature Request template](https://github.com/jarvy-dev/jarvy/issues/new?template=feature_request.yml)
3. Describe the problem you're trying to solve
4. Propose a specific solution

### Requesting New Tools

1. Check [existing tool requests](https://github.com/jarvy-dev/jarvy/issues?q=is%3Aissue+label%3Atool-request)
2. Use the [Tool Request template](https://github.com/jarvy-dev/jarvy/issues/new?template=tool_request.yml)
3. Include installation methods for all platforms

### Contributing Code

#### Adding a New Tool

The easiest way to contribute is by adding support for a new tool:

```bash
# Scaffold a new tool
cargo run -p cargo-jarvy -- new-tool <tool-name>
```

This creates:
- `src/tools/<tool>/mod.rs`
- `src/tools/<tool>/<tool>.rs`

Edit the tool definition:

```rust
use crate::define_tool;

define_tool!(MYTOOL, {
    command: "mytool",
    macos: { brew: "mytool" },
    linux: { uniform: "mytool" },
    windows: { winget: "publisher.mytool" },
});
```

Register in `src/tools/mod.rs` and submit a PR.

#### Code Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Run lints: `cargo clippy --all-features -- -D warnings`
6. Format: `cargo fmt --all`
7. Commit with conventional commit messages
8. Push and open a Pull Request

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add kubectl tool support
fix: resolve Windows path handling bug
docs: update installation guide
chore: update dependencies
refactor: simplify config parsing
test: add integration tests for roles
```

## Code Style

- Follow Rust 2024 edition idioms
- Use `cargo fmt` for formatting
- All `cargo clippy` warnings must be resolved
- Prefer stdlib and existing dependencies over new crates
- Write clear, self-documenting code
- Add comments for complex logic

## Testing

- Write unit tests for new functionality
- Integration tests go in `/tests/`
- Run `cargo test --verbose -- --show-output`
- Use `JARVY_TEST_MODE=1` to disable interactive prompts

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Fill out the PR template completely
4. Link related issues
5. Request review from maintainers
6. Address review feedback promptly

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- `jarvy contributors` command
- Release notes

See our [Contributor Recognition Program](./CONTRIBUTORS.md) for badge criteria.

## Getting Help

- [GitHub Discussions](https://github.com/jarvy-dev/jarvy/discussions) for questions
- [SUPPORT.md](./SUPPORT.md) for support options
- Check `jarvy faq` for common questions

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.

---

Thank you for contributing to Jarvy!
