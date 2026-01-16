# PRD-012: Release Process and Multi-Platform Distribution

## Overview

Establish a professional release process and distribute Jarvy to all major package managers (Homebrew, apt, dnf, pacman, winget, Chocolatey, cargo) with automated CI/CD pipelines, binary builds for all platforms, and proper versioning.

## Problem Statement

Jarvy currently has no formal release process or distribution strategy:

1. **No published releases**: Users must build from source
2. **No package manager presence**: Not available via brew, apt, winget, etc.
3. **No binary downloads**: No pre-compiled binaries for any platform
4. **No versioning strategy**: No semantic versioning, tags, or changelog
5. **No install script**: No `curl | bash` quick install option
6. **Manual process**: No automation for releases

## Goals

1. **One-command install** on any platform
2. **Available everywhere**: brew, apt, dnf, pacman, winget, choco, cargo
3. **Automated releases**: Tag → Build → Publish pipeline
4. **Professional versioning**: SemVer, changelogs, release notes
5. **Cross-platform binaries**: macOS (Intel/ARM), Linux (x64/ARM), Windows (x64)

## Target Package Managers

| Platform | Package Manager | Priority | Complexity |
|----------|-----------------|----------|------------|
| macOS | Homebrew | P0 | Medium |
| macOS | Homebrew Cask | P2 | Low |
| Linux | apt (deb) | P0 | High |
| Linux | dnf (rpm) | P1 | High |
| Linux | pacman (AUR) | P1 | Medium |
| Linux | apk (Alpine) | P2 | Medium |
| Linux | snap | P2 | Medium |
| Linux | flatpak | P3 | High |
| Windows | winget | P0 | Medium |
| Windows | Chocolatey | P1 | Medium |
| Windows | scoop | P2 | Low |
| Cross-platform | cargo | P0 | Low |
| Cross-platform | GitHub Releases | P0 | Low |
| Cross-platform | Install script | P0 | Medium |

## Requirements

### Functional Requirements

#### FR-1: Version Management
1. Semantic versioning (MAJOR.MINOR.PATCH)
2. Version stored in `Cargo.toml`
3. Git tags for each release (v1.0.0 format)
4. Automatic changelog generation from conventional commits
5. Pre-release versions supported (v1.0.0-beta.1)

#### FR-2: Binary Builds
1. **macOS**: x86_64-apple-darwin, aarch64-apple-darwin (Universal binary optional)
2. **Linux**: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86_64-unknown-linux-musl
3. **Windows**: x86_64-pc-windows-msvc, x86_64-pc-windows-gnu
4. Static linking where possible for portability
5. Stripped binaries for smaller size
6. Compressed archives (.tar.gz for Unix, .zip for Windows)

#### FR-3: GitHub Releases
1. Automatic release creation on version tag
2. Binary artifacts attached to release
3. Generated release notes from commits
4. SHA256 checksums for all artifacts
5. Installation instructions in release body

#### FR-4: Homebrew Distribution
1. Homebrew tap repository: `jarvy-dev/homebrew-tap`
2. Formula with binary bottles for Intel and ARM Mac
3. Linux Homebrew support
4. Automatic formula updates on release

#### FR-5: Linux Package Repositories
1. **Debian/Ubuntu (.deb)**:
   - APT repository hosted (GitHub Pages or Cloudflare R2)
   - GPG signed packages
   - Support for Ubuntu 20.04+, Debian 11+
2. **Fedora/RHEL (.rpm)**:
   - DNF/YUM repository
   - GPG signed packages
   - Support for Fedora 38+, RHEL 8+
3. **Arch Linux (AUR)**:
   - PKGBUILD in AUR
   - Both `jarvy` (build from source) and `jarvy-bin` (binary)

#### FR-6: Windows Distribution
1. **winget**:
   - Manifest in winget-pkgs repository
   - Automatic PR on release
2. **Chocolatey**:
   - Package in Chocolatey community repository
   - Automatic update on release
3. **Scoop**:
   - Manifest in scoop extras bucket

#### FR-7: Cargo/crates.io
1. Publish to crates.io
2. Proper metadata in Cargo.toml
3. Documentation on docs.rs

#### FR-8: Install Scripts
1. **Unix**: `curl -fsSL https://jarvy.dev/install.sh | bash`
   - Detect OS and architecture
   - Download appropriate binary
   - Install to ~/.local/bin or /usr/local/bin
   - Add to PATH if needed
2. **Windows**: `irm https://jarvy.dev/install.ps1 | iex`
   - Download Windows binary
   - Install to appropriate location
   - Update PATH

### Non-Functional Requirements

1. **Security**: GPG-signed packages, checksums, HTTPS downloads
2. **Speed**: Binary downloads < 30 seconds on reasonable connection
3. **Reliability**: Multiple mirror/CDN support
4. **Automation**: Zero manual steps after tagging a release

## Architecture

### Release Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                      Developer Workflow                          │
├─────────────────────────────────────────────────────────────────┤
│  1. Merge PRs to main                                           │
│  2. Run: cargo release patch/minor/major                        │
│     - Bumps version in Cargo.toml                               │
│     - Generates CHANGELOG.md                                    │
│     - Creates git tag (v1.2.3)                                  │
│     - Pushes tag to GitHub                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GitHub Actions (on tag)                       │
├─────────────────────────────────────────────────────────────────┤
│  Job: Build Binaries (matrix)                                   │
│  ├── macOS x86_64                                               │
│  ├── macOS aarch64                                              │
│  ├── Linux x86_64 (glibc)                                       │
│  ├── Linux x86_64 (musl)                                        │
│  ├── Linux aarch64                                              │
│  ├── Windows x86_64                                             │
│  └── Generate SHA256 checksums                                  │
├─────────────────────────────────────────────────────────────────┤
│  Job: Create GitHub Release                                     │
│  ├── Generate release notes                                     │
│  ├── Upload binary artifacts                                    │
│  └── Upload checksums                                           │
├─────────────────────────────────────────────────────────────────┤
│  Job: Publish to Package Managers                               │
│  ├── crates.io (cargo publish)                                  │
│  ├── Homebrew (update formula)                                  │
│  ├── winget (submit PR)                                         │
│  ├── Chocolatey (push package)                                  │
│  ├── AUR (update PKGBUILD)                                      │
│  └── APT/RPM repos (upload packages)                            │
└─────────────────────────────────────────────────────────────────┘
```

### Repository Structure

```
jarvy/
├── Cargo.toml                    # Version source of truth
├── CHANGELOG.md                  # Auto-generated changelog
├── .github/
│   └── workflows/
│       ├── ci.yml                # PR checks
│       ├── release.yml           # Release pipeline
│       └── publish-packages.yml  # Package manager updates
├── dist/
│   ├── homebrew/
│   │   └── jarvy.rb              # Homebrew formula template
│   ├── debian/
│   │   ├── control               # Debian package metadata
│   │   ├── rules                 # Build rules
│   │   └── postinst              # Post-install script
│   ├── rpm/
│   │   └── jarvy.spec            # RPM spec file
│   ├── aur/
│   │   ├── PKGBUILD              # AUR build script
│   │   └── PKGBUILD-bin          # Binary package
│   ├── windows/
│   │   ├── winget.yaml           # Winget manifest template
│   │   └── chocolatey/
│   │       ├── jarvy.nuspec
│   │       └── tools/
│   │           └── chocolateyinstall.ps1
│   └── scripts/
│       ├── install.sh            # Unix install script
│       └── install.ps1           # Windows install script
└── release.toml                  # cargo-release configuration
```

## Implementation Plan

### Phase 1: Foundation (Week 1)

#### 1.1 Version Management Setup
```toml
# Cargo.toml additions
[package]
version = "0.1.0"
edition = "2024"
description = "Cross-platform development environment provisioner"
license = "MIT OR Apache-2.0"
repository = "https://github.com/jarvy-dev/jarvy"
homepage = "https://jarvy.dev"
documentation = "https://docs.rs/jarvy"
readme = "README.md"
keywords = ["devtools", "provisioning", "cli", "developer-experience"]
categories = ["command-line-utilities", "development-tools"]

[package.metadata.release]
sign-commit = false
sign-tag = false
push = true
publish = true
```

#### 1.2 Changelog Generation
```toml
# cliff.toml (git-cliff configuration)
[changelog]
header = "# Changelog\n\nAll notable changes to Jarvy.\n"
body = """
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
- {{ commit.message | upper_first }} ([{{ commit.id | truncate(length=7, end="") }}](https://github.com/jarvy-dev/jarvy/commit/{{ commit.id }}))
{% endfor %}
{% endfor %}
"""

[git]
conventional_commits = true
filter_unconventional = true
commit_parsers = [
  { message = "^feat", group = "Features" },
  { message = "^fix", group = "Bug Fixes" },
  { message = "^doc", group = "Documentation" },
  { message = "^perf", group = "Performance" },
  { message = "^refactor", group = "Refactoring" },
  { message = "^test", group = "Testing" },
  { message = "^chore", group = "Miscellaneous" },
]
```

### Phase 2: Binary Builds (Week 1-2)

#### 2.1 Release Workflow
```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-apple-darwin
            os: macos-latest
            archive: tar.gz
          - target: aarch64-apple-darwin
            os: macos-latest
            archive: tar.gz
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            archive: tar.gz
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive: zip

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package
        shell: bash
        run: |
          cd target/${{ matrix.target }}/release
          if [ "${{ matrix.archive }}" = "zip" ]; then
            7z a ../../../jarvy-${{ github.ref_name }}-${{ matrix.target }}.zip jarvy.exe
          else
            tar czvf ../../../jarvy-${{ github.ref_name }}-${{ matrix.target }}.tar.gz jarvy
          fi

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: jarvy-${{ matrix.target }}
          path: jarvy-${{ github.ref_name }}-${{ matrix.target }}.*

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Generate checksums
        run: |
          cd artifacts
          find . -name 'jarvy-*' -type f -exec sha256sum {} \; > SHA256SUMS

      - name: Create release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            artifacts/**/*
            artifacts/SHA256SUMS
          generate_release_notes: true
```

### Phase 3: Homebrew (Week 2)

#### 3.1 Homebrew Tap Setup
```ruby
# dist/homebrew/jarvy.rb
class Jarvy < Formula
  desc "Cross-platform development environment provisioner"
  homepage "https://jarvy.dev"
  version "VERSION_PLACEHOLDER"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_intel do
      url "https://github.com/jarvy-dev/jarvy/releases/download/vVERSION_PLACEHOLDER/jarvy-vVERSION_PLACEHOLDER-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_PLACEHOLDER_MACOS_X86"
    end
    on_arm do
      url "https://github.com/jarvy-dev/jarvy/releases/download/vVERSION_PLACEHOLDER/jarvy-vVERSION_PLACEHOLDER-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_PLACEHOLDER_MACOS_ARM"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/jarvy-dev/jarvy/releases/download/vVERSION_PLACEHOLDER/jarvy-vVERSION_PLACEHOLDER-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_PLACEHOLDER_LINUX_X86"
    end
    on_arm do
      url "https://github.com/jarvy-dev/jarvy/releases/download/vVERSION_PLACEHOLDER/jarvy-vVERSION_PLACEHOLDER-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_PLACEHOLDER_LINUX_ARM"
    end
  end

  def install
    bin.install "jarvy"

    # Generate shell completions
    generate_completions_from_executable(bin/"jarvy", "completions")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/jarvy --version")
  end
end
```

### Phase 4: Linux Packages (Week 2-3)

#### 4.1 Debian Package
```
# dist/debian/control
Package: jarvy
Version: VERSION_PLACEHOLDER
Section: devel
Priority: optional
Architecture: amd64
Depends: libc6 (>= 2.17)
Maintainer: Jarvy Team <team@jarvy.dev>
Description: Cross-platform development environment provisioner
 Jarvy provisions development environments from a jarvy.toml config file.
 It installs tools via native package managers (brew, apt, dnf, winget).
Homepage: https://jarvy.dev
```

#### 4.2 RPM Spec
```spec
# dist/rpm/jarvy.spec
Name:           jarvy
Version:        VERSION_PLACEHOLDER
Release:        1%{?dist}
Summary:        Cross-platform development environment provisioner
License:        MIT or Apache-2.0
URL:            https://jarvy.dev
Source0:        https://github.com/jarvy-dev/jarvy/releases/download/v%{version}/jarvy-v%{version}-x86_64-unknown-linux-gnu.tar.gz

%description
Jarvy provisions development environments from a jarvy.toml config file.
It installs tools via native package managers (brew, apt, dnf, winget).

%prep
%setup -q -c

%install
install -Dm755 jarvy %{buildroot}%{_bindir}/jarvy

%files
%{_bindir}/jarvy
```

#### 4.3 AUR PKGBUILD
```bash
# dist/aur/PKGBUILD-bin
pkgname=jarvy-bin
pkgver=VERSION_PLACEHOLDER
pkgrel=1
pkgdesc="Cross-platform development environment provisioner"
arch=('x86_64' 'aarch64')
url="https://jarvy.dev"
license=('MIT' 'Apache-2.0')
provides=('jarvy')
conflicts=('jarvy')
source_x86_64=("https://github.com/jarvy-dev/jarvy/releases/download/v${pkgver}/jarvy-v${pkgver}-x86_64-unknown-linux-gnu.tar.gz")
source_aarch64=("https://github.com/jarvy-dev/jarvy/releases/download/v${pkgver}/jarvy-v${pkgver}-aarch64-unknown-linux-gnu.tar.gz")
sha256sums_x86_64=('SHA256_PLACEHOLDER')
sha256sums_aarch64=('SHA256_PLACEHOLDER')

package() {
    install -Dm755 jarvy "$pkgdir/usr/bin/jarvy"
}
```

### Phase 5: Windows Packages (Week 3)

#### 5.1 Winget Manifest
```yaml
# dist/windows/winget.yaml
PackageIdentifier: Jarvy.Jarvy
PackageVersion: VERSION_PLACEHOLDER
PackageLocale: en-US
Publisher: Jarvy
PackageName: Jarvy
PackageUrl: https://jarvy.dev
License: MIT OR Apache-2.0
ShortDescription: Cross-platform development environment provisioner
Installers:
  - Architecture: x64
    InstallerType: zip
    InstallerUrl: https://github.com/jarvy-dev/jarvy/releases/download/vVERSION_PLACEHOLDER/jarvy-vVERSION_PLACEHOLDER-x86_64-pc-windows-msvc.zip
    InstallerSha256: SHA256_PLACEHOLDER
ManifestType: singleton
ManifestVersion: 1.4.0
```

#### 5.2 Chocolatey Package
```xml
<!-- dist/windows/chocolatey/jarvy.nuspec -->
<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2015/06/nuspec.xsd">
  <metadata>
    <id>jarvy</id>
    <version>VERSION_PLACEHOLDER</version>
    <title>Jarvy</title>
    <authors>Jarvy Team</authors>
    <projectUrl>https://jarvy.dev</projectUrl>
    <licenseUrl>https://github.com/jarvy-dev/jarvy/blob/main/LICENSE</licenseUrl>
    <requireLicenseAcceptance>false</requireLicenseAcceptance>
    <projectSourceUrl>https://github.com/jarvy-dev/jarvy</projectSourceUrl>
    <docsUrl>https://jarvy.dev/docs</docsUrl>
    <bugTrackerUrl>https://github.com/jarvy-dev/jarvy/issues</bugTrackerUrl>
    <tags>devtools provisioning cli developer-experience</tags>
    <summary>Cross-platform development environment provisioner</summary>
    <description>Jarvy provisions development environments from a jarvy.toml config file.</description>
  </metadata>
  <files>
    <file src="tools\**" target="tools" />
  </files>
</package>
```

### Phase 6: Install Scripts (Week 3)

#### 6.1 Unix Install Script
```bash
#!/bin/bash
# dist/scripts/install.sh
set -euo pipefail

JARVY_VERSION="${JARVY_VERSION:-latest}"
JARVY_INSTALL_DIR="${JARVY_INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and architecture
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin) os="apple-darwin" ;;
        Linux) os="unknown-linux-gnu" ;;
        *) echo "Unsupported OS: $os"; exit 1 ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "Unsupported architecture: $arch"; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

# Get latest version from GitHub
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/jarvy-dev/jarvy/releases/latest" | \
        grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/'
}

main() {
    local platform version url
    platform="$(detect_platform)"

    if [ "$JARVY_VERSION" = "latest" ]; then
        version="$(get_latest_version)"
    else
        version="$JARVY_VERSION"
    fi

    url="https://github.com/jarvy-dev/jarvy/releases/download/v${version}/jarvy-v${version}-${platform}.tar.gz"

    echo "Installing Jarvy v${version} for ${platform}..."

    mkdir -p "$JARVY_INSTALL_DIR"
    curl -fsSL "$url" | tar -xz -C "$JARVY_INSTALL_DIR"
    chmod +x "$JARVY_INSTALL_DIR/jarvy"

    echo "Jarvy installed to $JARVY_INSTALL_DIR/jarvy"

    # Check if in PATH
    if ! command -v jarvy &>/dev/null; then
        echo ""
        echo "Add to PATH by running:"
        echo '  export PATH="$HOME/.local/bin:$PATH"'
        echo ""
        echo "Or add to your shell profile:"
        echo '  echo '"'"'export PATH="$HOME/.local/bin:$PATH"'"'"' >> ~/.bashrc'
    fi
}

main "$@"
```

#### 6.2 Windows Install Script
```powershell
# dist/scripts/install.ps1
$ErrorActionPreference = 'Stop'

$JarvyVersion = if ($env:JARVY_VERSION) { $env:JARVY_VERSION } else { "latest" }
$InstallDir = if ($env:JARVY_INSTALL_DIR) { $env:JARVY_INSTALL_DIR } else { "$env:LOCALAPPDATA\Programs\jarvy" }

function Get-LatestVersion {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/jarvy-dev/jarvy/releases/latest"
    return $release.tag_name -replace '^v', ''
}

function Install-Jarvy {
    if ($JarvyVersion -eq "latest") {
        $version = Get-LatestVersion
    } else {
        $version = $JarvyVersion
    }

    $url = "https://github.com/jarvy-dev/jarvy/releases/download/v$version/jarvy-v$version-x86_64-pc-windows-msvc.zip"
    $zipPath = "$env:TEMP\jarvy.zip"

    Write-Host "Installing Jarvy v$version..."

    # Download
    Invoke-WebRequest -Uri $url -OutFile $zipPath

    # Extract
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path $zipPath -DestinationPath $InstallDir -Force
    Remove-Item $zipPath

    Write-Host "Jarvy installed to $InstallDir\jarvy.exe"

    # Add to PATH
    $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("PATH", "$userPath;$InstallDir", "User")
        Write-Host "Added $InstallDir to PATH"
        Write-Host "Restart your terminal to use jarvy"
    }
}

Install-Jarvy
```

### Phase 7: Cargo/crates.io (Week 3)

#### 7.1 Cargo.toml Updates
```toml
[package]
name = "jarvy"
version = "1.0.0"
edition = "2024"
rust-version = "1.75"
description = "Cross-platform development environment provisioner"
license = "MIT OR Apache-2.0"
repository = "https://github.com/jarvy-dev/jarvy"
homepage = "https://jarvy.dev"
documentation = "https://docs.rs/jarvy"
readme = "README.md"
keywords = ["devtools", "provisioning", "cli", "developer-experience", "setup"]
categories = ["command-line-utilities", "development-tools"]
include = [
    "src/**/*",
    "Cargo.toml",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "README.md",
]

[badges]
maintenance = { status = "actively-developed" }
```

## Release Checklist

### Pre-Release
- [ ] All tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy`)
- [ ] Documentation builds (`cargo doc`)
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml

### Release
- [ ] Create and push tag (`git tag v1.0.0 && git push --tags`)
- [ ] Verify GitHub Actions builds complete
- [ ] Verify GitHub Release created with all artifacts
- [ ] Verify checksums file present

### Post-Release Package Updates
- [ ] Homebrew formula updated
- [ ] crates.io published
- [ ] winget manifest PR submitted
- [ ] Chocolatey package pushed
- [ ] AUR PKGBUILD updated
- [ ] APT repository updated
- [ ] RPM repository updated
- [ ] Install scripts tested

## Success Metrics

| Metric | Target |
|--------|--------|
| Package managers supported | 8+ |
| Platforms with binaries | 6 |
| Install time (network) | < 30s |
| Release automation | 100% |
| Time from tag to published | < 15 min |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Package manager rejection | Medium | High | Follow guidelines, provide quality packages |
| Build failures on new platforms | Medium | Medium | Extensive CI matrix, cross-compilation |
| GPG key management | Low | High | Secure key storage, rotation policy |
| CDN/hosting costs | Low | Medium | GitHub Releases free, consider sponsors |

## Dependencies

- `cargo-release` - Version management
- `git-cliff` - Changelog generation
- `cross` - Cross-compilation
- GitHub Actions runners (macOS, Linux, Windows)

## Effort Estimate

| Phase | Task | Effort |
|-------|------|--------|
| 1 | Version management, changelog | 1 day |
| 2 | Binary builds CI | 2 days |
| 3 | GitHub Releases automation | 0.5 days |
| 4 | Homebrew tap | 1 day |
| 5 | Linux packages (deb, rpm) | 2 days |
| 6 | AUR package | 0.5 days |
| 7 | Windows packages (winget, choco) | 1.5 days |
| 8 | Install scripts | 1 day |
| 9 | crates.io publishing | 0.5 days |
| 10 | Testing and polish | 1 day |
| **Total** | | **~11 days** |

## Files to Create

```
.github/workflows/release.yml
.github/workflows/publish-packages.yml
dist/
├── homebrew/jarvy.rb
├── debian/
│   ├── control
│   ├── rules
│   └── postinst
├── rpm/jarvy.spec
├── aur/
│   ├── PKGBUILD
│   └── PKGBUILD-bin
├── windows/
│   ├── winget.yaml
│   └── chocolatey/
│       ├── jarvy.nuspec
│       └── tools/chocolateyinstall.ps1
└── scripts/
    ├── install.sh
    └── install.ps1
cliff.toml
release.toml
CHANGELOG.md
```
