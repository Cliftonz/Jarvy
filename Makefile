# Common contributor entrypoints. `make setup` is the only command a new
# developer should need on a clean laptop.

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

.PHONY: help setup bootstrap doctor drift fmt lint test test-sandbox test-install-pipeline e2e-pull-images build clean

# Cross-build target the container integration tests mount into linux
# containers. arm64 chosen so Apple Silicon hosts run containers
# natively under Docker Desktop (no QEMU). Linux CI hits a different
# code path (its native binary is already a linux ELF) and skips this
# cross-build entirely.
#
# Debug profile (not release): the release profile uses `lto = "fat"`
# + `codegen-units = 1`, which can OOM-kill the rustc linker inside
# Docker Desktop's default 4-8 GB cgroup on large crates. The test
# harness only needs a runnable binary, not an optimized one.
LINUX_TEST_TARGET ?= aarch64-unknown-linux-gnu
LINUX_TEST_BIN := target/$(LINUX_TEST_TARGET)/debug/jarvy

# Derive libc family from the target triple. The Alpine green-path test
# only runs when JARVY_BIN_LIBC=musl; the glibc-on-alpine expected-fail
# test only runs when JARVY_BIN_LIBC=glibc. Override by passing a
# different LINUX_TEST_TARGET (e.g. `aarch64-unknown-linux-musl`).
ifeq ($(findstring musl,$(LINUX_TEST_TARGET)),musl)
JARVY_BIN_LIBC := musl
else
JARVY_BIN_LIBC := glibc
endif

# Image set the install-pipeline matrix exercises. Kept in sync with
# the digest constants in tests/e2e_install_pipeline.rs. Pre-pulling
# parallelizes outside the cargo-test timer and survives Docker Hub
# throttle better than letting testcontainers race for them.
E2E_IMAGES := ubuntu:22.04 ubuntu:24.04 debian:bookworm-slim fedora:40 \
              rockylinux:9 amazonlinux:2 archlinux:latest \
              opensuse/leap:15.6 alpine:3.20

help:  ## Show available targets
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

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

test-sandbox: $(LINUX_TEST_BIN)  ## Cross-build linux jarvy + run sandbox integration tests against Docker
	JARVY_TEST_BIN=$(abspath $(LINUX_TEST_BIN)) cargo test --test sandbox_integration -- --nocapture

# `--test-threads` is overridable: 8 parallel container spins swamp
# Docker Desktop's default 4-8 GB cgroup on laptops, but fatter CI
# runners can use the full matrix. Default 4 trades CI wall-clock for
# laptop survivability — bump with `JARVY_E2E_THREADS=8 make ...`.
test-install-pipeline: $(LINUX_TEST_BIN) e2e-pull-images  ## Smoke-test the locally built jarvy against every supported Linux distro in Docker
	JARVY_E2E_INSTALL=1 \
	JARVY_TEST_BIN=$(abspath $(LINUX_TEST_BIN)) \
	JARVY_BIN_LIBC=$(JARVY_BIN_LIBC) \
		cargo test --test e2e_install_pipeline -- --nocapture --test-threads=$${JARVY_E2E_THREADS:-4}

# Pull all distro images in parallel with a one-shot retry. Pre-pull is
# best-effort — some images (e.g. archlinux) ship x86_64-only manifests
# and will fail on Apple Silicon. The per-distro test panics with a
# clear breadcrumb if its image is unavailable, so this target never
# fails the whole make invocation.
e2e-pull-images:  ## Pre-pull the install-pipeline image set (best-effort, parallel)
	@printf '%s\n' $(E2E_IMAGES) | xargs -P 8 -I{} sh -c \
		'docker pull {} >/dev/null 2>&1 || (sleep 5 && docker pull {} >/dev/null 2>&1) \
		|| echo "  (skipped — no manifest for current arch?) {}" >&2' || true

$(LINUX_TEST_BIN):
	@command -v cross >/dev/null 2>&1 || { \
		echo "cross not found. Install with:"; \
		echo "  cargo install cross --git https://github.com/cross-rs/cross"; \
		exit 1; \
	}
	cross build --target $(LINUX_TEST_TARGET) --bin jarvy

build:  ## Release build
	@cargo build --release

clean:  ## Clean build artifacts
	@cargo clean
