# PRD-020: Security Scanning Infrastructure

## Overview

Implement comprehensive security scanning beyond GitHub Dependabot to ensure supply chain security, detect vulnerabilities early, enforce license compliance, and establish security best practices for the Jarvy project.

## Problem Statement

Dependabot provides basic dependency vulnerability scanning but has significant limitations:

1. **Limited scope**: Only scans Cargo.lock for known CVEs, misses many vulnerability classes
2. **No SAST**: No static analysis to catch security bugs in our own code
3. **No secret scanning**: Credentials can leak into commits or releases
4. **No license compliance**: Dependencies may introduce incompatible licenses
5. **No SBOM**: No Software Bill of Materials for downstream consumers
6. **No supply chain verification**: No artifact signing or provenance attestation
7. **Reactive only**: Alerts after vulnerabilities exist, no proactive prevention
8. **No container scanning**: If we add Docker support, images need scanning

## Goals

1. **Defense in depth**: Multiple overlapping security tools for comprehensive coverage
2. **Shift left**: Catch security issues before they reach main branch
3. **Supply chain security**: Sign releases, generate SBOMs, verify dependencies
4. **License compliance**: Ensure all dependencies have compatible licenses
5. **Automated enforcement**: Fail CI on security issues, not just warn
6. **Transparency**: Publish security posture via OpenSSF Scorecard

## Non-Goals

- Runtime application security monitoring (Jarvy is a CLI tool)
- Penetration testing infrastructure
- Bug bounty program management
- SOC 2 / compliance certification processes

## Requirements

### Functional Requirements

#### FR-1: Static Application Security Testing (SAST)

Scan Jarvy's Rust source code for security vulnerabilities.

| Tool | Purpose | Priority |
|------|---------|----------|
| `cargo-audit` | CVE database scanning | P0 |
| `cargo-deny` | License + advisory + ban checks | P0 |
| Semgrep | Custom security rules for Rust | P1 |
| CodeQL | GitHub-native semantic analysis | P1 |
| `cargo-geiger` | Unsafe code detection | P2 |

**Checks to enforce:**
- No known CVEs in dependencies
- No unsafe code without explicit justification (#[allow(unsafe)] with comment)
- No hardcoded secrets or credentials
- No command injection vulnerabilities (shell escaping)
- No path traversal vulnerabilities
- No insecure random number generation
- No deprecated cryptographic functions

#### FR-2: Secret Scanning

Prevent credentials from leaking into the repository or releases.

| Tool | Purpose | Priority |
|------|---------|----------|
| GitHub Secret Scanning | Native secret detection | P0 |
| Gitleaks | Pre-commit + CI scanning | P0 |
| TruffleHog | Historical commit scanning | P2 |

**Patterns to detect:**
- API keys (AWS, GCP, Azure, GitHub, etc.)
- Private keys (RSA, SSH, GPG)
- Database connection strings
- OAuth tokens and secrets
- Webhook URLs with secrets
- Generic high-entropy strings

#### FR-3: Dependency Security

Deep scanning of the dependency tree beyond CVEs.

| Tool | Purpose | Priority |
|------|---------|----------|
| `cargo-audit` | RustSec advisory database | P0 |
| `cargo-deny` | Comprehensive dependency policy | P0 |
| `cargo-outdated` | Outdated dependency detection | P1 |
| `cargo-vet` | Dependency auditing | P2 |
| Snyk | Commercial vuln database | P3 |

**Policy enforcement:**
- Block dependencies with known vulnerabilities (RUSTSEC advisories)
- Block dependencies with incompatible licenses
- Block specific problematic crates (unmaintained, typosquat, etc.)
- Warn on dependencies not audited by trusted parties
- Require justification for yanked crate versions

#### FR-4: License Compliance

Ensure all dependencies have OSS licenses compatible with MIT/Apache-2.0.

**Allowed licenses:**
- MIT
- Apache-2.0
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Zlib
- CC0-1.0
- Unlicense

**Blocked licenses:**
- GPL (any version) - copyleft incompatible
- AGPL - network copyleft
- SSPL - server-side restrictions
- Commons Clause - commercial restrictions
- Proprietary / No license

**Actions:**
- Fail CI if dependency introduces blocked license
- Generate license report for releases (NOTICES file)
- Maintain exceptions list with justification

#### FR-5: Software Bill of Materials (SBOM)

Generate machine-readable inventory of all components.

| Format | Standard | Priority |
|--------|----------|----------|
| SPDX | ISO/IEC 5962:2021 | P0 |
| CycloneDX | OWASP standard | P1 |

**SBOM contents:**
- All direct and transitive dependencies
- Version information
- License information
- Supplier/author information
- Cryptographic hashes
- Vulnerability status (VEX)

**Distribution:**
- Attach SBOM to GitHub Releases
- Include in published crates.io package
- Publish to dependency-track (if self-hosted)

#### FR-6: Supply Chain Security

Sign artifacts and provide provenance attestation.

| Tool | Purpose | Priority |
|------|---------|----------|
| Sigstore/cosign | Keyless artifact signing | P0 |
| SLSA | Build provenance attestation | P1 |
| GitHub Artifact Attestations | Native provenance | P1 |

**Signing targets:**
- Release binaries (all platforms)
- Container images (if applicable)
- SBOMs
- Checksums file

**Provenance claims (SLSA Level 2+):**
- Build platform (GitHub Actions)
- Source repository and commit
- Build instructions (workflow file)
- Builder identity

#### FR-7: Security Scorecard

Public security posture reporting via OpenSSF Scorecard.

**Scorecard checks to pass:**
- Binary-Artifacts: No checked-in binaries
- Branch-Protection: Require reviews, status checks
- CI-Tests: Tests run on PRs
- CII-Best-Practices: OpenSSF badge
- Code-Review: All changes reviewed
- Contributors: Multiple contributors
- Dangerous-Workflow: No dangerous patterns
- Dependency-Update-Tool: Dependabot enabled
- Fuzzing: Fuzz testing in place
- License: License file present
- Maintained: Recent commits
- Pinned-Dependencies: Pinned GitHub Actions
- Packaging: Follows packaging best practices
- SAST: Static analysis enabled
- Security-Policy: SECURITY.md present
- Signed-Releases: Releases signed
- Token-Permissions: Minimal token permissions
- Vulnerabilities: No known vulns

**Target score:** 8.0+ out of 10

### Non-Functional Requirements

1. **Performance**: Security scans complete in < 5 minutes
2. **Reliability**: No flaky failures from security tooling
3. **Maintainability**: Security configs in declarative files
4. **Transparency**: Public security reports and badges

## Architecture

### CI/CD Pipeline Integration

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Pull Request                                  │
├─────────────────────────────────────────────────────────────────────┤
│  Pre-commit (local)                                                  │
│  ├── gitleaks (secret scanning)                                     │
│  └── cargo fmt/clippy (optional security lints)                     │
├─────────────────────────────────────────────────────────────────────┤
│  CI Pipeline (GitHub Actions)                                        │
│  ├── cargo-audit (CVE scan)                                         │
│  ├── cargo-deny (license + advisories + bans)                       │
│  ├── gitleaks (full repo scan)                                      │
│  ├── Semgrep (SAST rules)                                           │
│  ├── CodeQL (semantic analysis)                                     │
│  └── cargo-geiger (unsafe audit)                                    │
├─────────────────────────────────────────────────────────────────────┤
│  Required Status Checks                                              │
│  ├── security/audit (cargo-audit)                                   │
│  ├── security/deny (cargo-deny)                                     │
│  ├── security/secrets (gitleaks)                                    │
│  └── security/sast (semgrep + codeql)                               │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Release Pipeline                             │
├─────────────────────────────────────────────────────────────────────┤
│  Pre-release Checks                                                  │
│  ├── All security checks pass                                       │
│  ├── No new vulnerabilities since last release                      │
│  └── License report generated                                       │
├─────────────────────────────────────────────────────────────────────┤
│  Build & Sign                                                        │
│  ├── Build binaries (cross-platform)                                │
│  ├── Generate SBOM (SPDX + CycloneDX)                               │
│  ├── Sign with Sigstore (cosign)                                    │
│  └── Generate SLSA provenance                                       │
├─────────────────────────────────────────────────────────────────────┤
│  Publish                                                             │
│  ├── GitHub Release with signatures                                 │
│  ├── Attach SBOM artifacts                                          │
│  ├── Attach provenance attestation                                  │
│  └── Update Scorecard badge                                         │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Scheduled Scans (Daily/Weekly)                  │
├─────────────────────────────────────────────────────────────────────┤
│  ├── Dependency freshness check                                     │
│  ├── New CVE detection (RustSec, NVD)                               │
│  ├── OpenSSF Scorecard update                                       │
│  └── TruffleHog historical scan                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### File Structure

```
jarvy/
├── .github/
│   └── workflows/
│       ├── security.yml          # Security-focused CI
│       ├── scorecard.yml         # OpenSSF Scorecard
│       └── release.yml           # Signing + SBOM
├── deny.toml                     # cargo-deny configuration
├── audit.toml                    # cargo-audit configuration
├── .gitleaks.toml                # Gitleaks configuration
├── .semgrep.yml                  # Semgrep rules
├── SECURITY.md                   # Security policy
├── NOTICES                       # Third-party license notices
└── scripts/
    └── security/
        ├── verify-signatures.sh  # Verify release signatures
        └── generate-sbom.sh      # SBOM generation helper
```

## Implementation Plan

### Phase 1: Foundation (P0)

#### 1.1 cargo-deny Configuration

```toml
# deny.toml
[graph]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"
ignore = [
    # Add ignored advisories with justification
]

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "CC0-1.0",
    "Unlicense",
    "MPL-2.0",  # Weak copyleft, file-level
]
copyleft = "deny"
exceptions = [
    # Add specific crate exceptions with justification
]

[licenses.private]
ignore = false
registries = []

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"
workspace-default-features = "allow"
external-default-features = "allow"

skip = [
    # Skip specific crate versions with justification
]

deny = [
    # Explicitly banned crates
    # { name = "openssl", wrappers = ["openssl-sys"] },  # Prefer rustls
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

#### 1.2 Security CI Workflow

```yaml
# .github/workflows/security.yml
name: Security

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 6 * * *'  # Daily at 6 AM UTC

permissions:
  contents: read
  security-events: write

jobs:
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run cargo-audit
        run: cargo audit --deny warnings

  deny:
    name: Dependency Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-deny
        uses: EmbarkStudios/cargo-deny-action@v1
        with:
          command: check all

  secrets:
    name: Secret Scanning
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Gitleaks
        uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  semgrep:
    name: SAST (Semgrep)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Semgrep
        uses: semgrep/semgrep-action@v1
        with:
          config: >-
            p/rust
            p/security-audit
            p/secrets
          generateSarif: true

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: semgrep.sarif
        if: always()

  codeql:
    name: SAST (CodeQL)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Initialize CodeQL
        uses: github/codeql-action/init@v3
        with:
          languages: rust
          queries: security-extended

      - name: Build
        run: cargo build --release

      - name: Perform CodeQL Analysis
        uses: github/codeql-action/analyze@v3
```

#### 1.3 Gitleaks Configuration

```toml
# .gitleaks.toml
title = "Jarvy Gitleaks Configuration"

[extend]
useDefault = true

[[rules]]
id = "jarvy-api-key"
description = "Jarvy API Key"
regex = '''jarvy[_-]?api[_-]?key['\"]?\s*[:=]\s*['\"]?([a-zA-Z0-9]{32,})'''
secretGroup = 1

[allowlist]
description = "Global allowlist"
paths = [
    '''\.gitleaks\.toml$''',
    '''deny\.toml$''',
    '''SECURITY\.md$''',
    '''docs/.*\.md$''',
]
```

#### 1.4 SECURITY.md

```markdown
# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via GitHub's private vulnerability reporting:
https://github.com/jarvy-dev/jarvy/security/advisories/new

Or email: security@jarvy.dev

You should receive a response within 48 hours. If for some reason you do not,
please follow up via email to ensure we received your original message.

Please include:
- Type of issue (buffer overflow, command injection, etc.)
- Full paths of source file(s) related to the issue
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

## Security Measures

This project implements:
- Static Application Security Testing (SAST) via Semgrep and CodeQL
- Dependency vulnerability scanning via cargo-audit and cargo-deny
- Secret scanning via Gitleaks
- License compliance checking
- Signed releases with Sigstore
- SBOM generation (SPDX and CycloneDX)
- OpenSSF Scorecard reporting

## Security Acknowledgments

We thank the following individuals for responsibly disclosing security issues:

(None yet)
```

### Phase 2: Supply Chain Security (P0-P1)

#### 2.1 Release Signing with Sigstore

```yaml
# Addition to .github/workflows/release.yml
  sign:
    name: Sign Artifacts
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
      id-token: write  # Required for Sigstore
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Install cosign
        uses: sigstore/cosign-installer@v3

      - name: Sign artifacts
        run: |
          for file in artifacts/**/*; do
            if [[ -f "$file" && ! "$file" =~ \.(sig|pem)$ ]]; then
              cosign sign-blob --yes "$file" \
                --output-signature "${file}.sig" \
                --output-certificate "${file}.pem"
            fi
          done

      - name: Upload signatures
        uses: actions/upload-artifact@v4
        with:
          name: signatures
          path: |
            artifacts/**/*.sig
            artifacts/**/*.pem
```

#### 2.2 SBOM Generation

```yaml
  sbom:
    name: Generate SBOM
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-sbom
        run: cargo install cargo-sbom

      - name: Generate SPDX SBOM
        run: |
          cargo sbom --output-format spdx_json_2_3 > sbom.spdx.json

      - name: Generate CycloneDX SBOM
        run: |
          cargo install cargo-cyclonedx
          cargo cyclonedx --format json > sbom.cdx.json

      - name: Upload SBOMs
        uses: actions/upload-artifact@v4
        with:
          name: sbom
          path: |
            sbom.spdx.json
            sbom.cdx.json
```

#### 2.3 SLSA Provenance

```yaml
  provenance:
    name: Generate SLSA Provenance
    needs: build
    permissions:
      actions: read
      id-token: write
      contents: write
    uses: slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v1.9.0
    with:
      base64-subjects: "${{ needs.build.outputs.hashes }}"
      upload-assets: true
```

### Phase 3: OpenSSF Scorecard (P1)

#### 3.1 Scorecard Workflow

```yaml
# .github/workflows/scorecard.yml
name: OpenSSF Scorecard

on:
  branch_protection_rule:
  schedule:
    - cron: '0 0 * * 1'  # Weekly on Monday
  push:
    branches: [main]

permissions: read-all

jobs:
  analysis:
    name: Scorecard Analysis
    runs-on: ubuntu-latest
    permissions:
      security-events: write
      id-token: write
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false

      - name: Run Scorecard
        uses: ossf/scorecard-action@v2
        with:
          results_file: results.sarif
          results_format: sarif
          publish_results: true

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: results.sarif
```

#### 3.2 Badge in README

```markdown
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/jarvy-dev/jarvy/badge)](https://securityscorecards.dev/viewer/?uri=github.com/jarvy-dev/jarvy)
```

### Phase 4: Advanced Scanning (P2)

#### 4.1 cargo-geiger (Unsafe Code Audit)

```yaml
  geiger:
    name: Unsafe Code Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-geiger
        run: cargo install cargo-geiger

      - name: Run geiger
        run: |
          cargo geiger --all-features --all-targets 2>&1 | tee geiger-report.txt
          # Fail if unsafe count exceeds threshold
          UNSAFE_COUNT=$(grep -oP 'Functions:\s+\K\d+' geiger-report.txt | tail -1)
          if [ "$UNSAFE_COUNT" -gt 0 ]; then
            echo "::warning::Found $UNSAFE_COUNT unsafe functions"
          fi
```

#### 4.2 cargo-vet (Supply Chain Audit)

```toml
# supply-chain/config.toml
[cargo-vet]
version = "0.8"

[imports.bytecode-alliance]
url = "https://raw.githubusercontent.com/aspect-build/aspect-cli/main/supply-chain/audits.toml"

[imports.mozilla]
url = "https://raw.githubusercontent.com/aspect-build/aspect-cli/main/supply-chain/audits.toml"

[policy.jarvy]
audit-as-crates-io = true
```

### Phase 5: Pre-commit Integration (P2)

#### 5.1 Pre-commit Config

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.18.0
    hooks:
      - id: gitleaks

  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --all -- --check
        language: system
        types: [rust]
        pass_filenames: false

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --all-features -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
```

## Success Metrics

| Metric | Target |
|--------|--------|
| OpenSSF Scorecard | 8.0+ |
| CVE response time | < 24 hours |
| False positive rate | < 5% |
| CI security scan time | < 5 minutes |
| Signed releases | 100% |
| SBOM coverage | 100% of releases |
| License compliance | 100% |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positives blocking PRs | Medium | Medium | Allowlist with justification |
| Tool performance degradation | Low | Medium | Caching, parallel execution |
| Sigstore outages | Low | Medium | Fallback signing keys |
| New CVE in critical dependency | Medium | High | Daily scans, rapid response process |
| License contamination | Low | High | cargo-deny strict enforcement |

## Dependencies

- `cargo-audit` - RustSec advisory scanning
- `cargo-deny` - License and dependency policy
- `cargo-sbom` - SPDX generation
- `cargo-cyclonedx` - CycloneDX generation
- `cargo-geiger` - Unsafe code detection
- Gitleaks - Secret scanning
- Semgrep - SAST rules
- CodeQL - Semantic analysis
- Sigstore/cosign - Artifact signing
- OpenSSF Scorecard - Security posture

## Effort Estimate

| Phase | Task | Effort |
|-------|------|--------|
| 1.1 | cargo-deny configuration | 0.5 days |
| 1.2 | Security CI workflow | 1 day |
| 1.3 | Gitleaks configuration | 0.5 days |
| 1.4 | SECURITY.md | 0.5 days |
| 2.1 | Sigstore signing | 1 day |
| 2.2 | SBOM generation | 0.5 days |
| 2.3 | SLSA provenance | 1 day |
| 3 | OpenSSF Scorecard | 0.5 days |
| 4 | Advanced scanning (geiger, vet) | 1 day |
| 5 | Pre-commit hooks | 0.5 days |
| | Testing and refinement | 1 day |
| **Total** | | **~8 days** |

## Files to Create/Modify

```
.github/workflows/security.yml       # New: Security CI
.github/workflows/scorecard.yml      # New: OpenSSF Scorecard
.github/workflows/release.yml        # Modify: Add signing + SBOM
deny.toml                            # New: cargo-deny config
.gitleaks.toml                       # New: Gitleaks config
.semgrep.yml                         # New: Custom Semgrep rules
.pre-commit-config.yaml              # New: Pre-commit hooks
SECURITY.md                          # New: Security policy
supply-chain/config.toml             # New: cargo-vet config (P2)
scripts/security/verify-signatures.sh # New: Signature verification
scripts/security/generate-sbom.sh     # New: SBOM helper
```

## References

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-deny Documentation](https://embarkstudios.github.io/cargo-deny/)
- [OpenSSF Scorecard](https://securityscorecards.dev/)
- [Sigstore Documentation](https://docs.sigstore.dev/)
- [SLSA Framework](https://slsa.dev/)
- [SPDX Specification](https://spdx.dev/)
- [CycloneDX Specification](https://cyclonedx.org/)
- [Semgrep Rust Rules](https://semgrep.dev/p/rust)
- [Gitleaks](https://gitleaks.io/)
