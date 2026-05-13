# Common contributor entrypoints. `make setup` is the only command a new
# developer should need on a clean laptop.

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

.PHONY: help setup bootstrap doctor drift fmt lint test test-sandbox build clean

# Cross-build target the sandbox integration tests mount into linux
# containers. arm64 chosen so Apple Silicon hosts run containers
# natively under Docker Desktop (no QEMU). Linux CI hits a different
# code path (its native binary is already a linux ELF) and skips this
# cross-build entirely.
#
# Debug profile (not release): the release profile uses `lto = "fat"`
# + `codegen-units = 1`, which can OOM-kill the rustc linker inside
# Docker Desktop's default 4-8 GB cgroup on large crates. The test
# harness only needs a runnable binary, not an optimized one.
SANDBOX_TARGET ?= aarch64-unknown-linux-gnu
SANDBOX_BIN := target/$(SANDBOX_TARGET)/debug/jarvy

help:  ## Show available targets
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: bootstrap  ## Install Jarvy if missing, then run jarvy setup (clean-laptop onboarding)

bootstrap:  ## Run the bootstrap script (idempotent)
	@./scripts/bootstrap.sh

doctor:  ## Verify environment health
	@jarvy doctor --extended

drift:  ## Check environment drift against the team baseline
	@jarvy drift check

fmt:  ## Format Rust code
	@cargo fmt --all

lint:  ## Run clippy with the same rules as CI
	@cargo clippy --all-features -- -D warnings

test:  ## Run the test suite
	@cargo test --verbose -- --show-output

test-sandbox: $(SANDBOX_BIN)  ## Cross-build linux jarvy + run sandbox integration tests against Docker
	JARVY_TEST_BIN=$(abspath $(SANDBOX_BIN)) cargo test --test sandbox_integration -- --nocapture

$(SANDBOX_BIN):
	@command -v cross >/dev/null 2>&1 || { \
		echo "cross not found. Install with:"; \
		echo "  cargo install cross --git https://github.com/cross-rs/cross"; \
		exit 1; \
	}
	cross build --target $(SANDBOX_TARGET) --bin jarvy

build:  ## Release build
	@cargo build --release

clean:  ## Clean build artifacts
	@cargo clean
