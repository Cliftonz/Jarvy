# Changelog

All notable changes to Jarvy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Features

- **tools:** Parallel version checking with rayon for 5x speedup
- **tools:** Batch package manager operations (brew install a b c instead of individual installs)
- **hooks:** 29 default post-install hooks for shell completion and configuration
- **services:** Docker Compose and Tilt backend support for service management
- **ci:** Auto-detection for 11 CI/CD providers with provider-specific output formatting
- **env:** Environment variable management with .env generation and shell rc updates

### Tools

- 97+ tools supported across macOS, Linux, and Windows
- Tools include: Node.js, Python, Go, Rust, Docker, Kubernetes, Terraform, AWS CLI, and more

### Infrastructure

- `define_tool!` macro for declarative tool definitions (~2000 lines reduced)
- Semantic version checking with proper semver operators
- Cross-platform shell detection and hook execution
