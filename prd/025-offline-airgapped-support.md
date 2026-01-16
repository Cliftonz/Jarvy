# PRD-025: Offline & Air-Gapped Support

## Overview

Enable Jarvy to work in environments with limited or no internet access, including air-gapped networks, by supporting local package mirrors, pre-downloaded bundles, and offline validation.

## Problem Statement

Jarvy currently assumes always-online connectivity:
- Cannot function in air-gapped environments (defense, healthcare, finance)
- No way to pre-download packages for offline use
- Config validation requires network for remote configs
- CI/CD in restricted networks cannot use Jarvy
- No support for local/private package mirrors

Many enterprise environments have restricted internet access for security reasons, making current Jarvy unusable.

## Evidence

- Defense contractors, healthcare, and financial services operate air-gapped networks
- Corporate networks often restrict outbound traffic
- CI runners in private subnets may lack internet
- Compliance requirements prohibit external downloads in production
- Users request "offline mode" for travel/unreliable networks

## Requirements

### Functional Requirements

1. **Local mirror support**: Configure private package mirrors
2. **Bundle mode**: Pre-download packages for offline installation
3. **Offline validation**: Validate configs without network
4. **Cache warming**: Pre-populate cache for team deployments
5. **Portable mode**: Self-contained Jarvy with tools

### Non-Functional Requirements

1. Works identically online and offline once cached
2. Clear error messages when offline and cache miss
3. Bundle size optimized (no unnecessary files)
4. Integrity verification for all cached packages
5. Supports all three platforms (macOS, Linux, Windows)

## Non-Goals

- Building our own package repository
- Hosting infrastructure for mirrors
- Automatic network switching
- Proxy configuration (future PRD)
- Package signing infrastructure

## Feature Specifications

### 1. Local Mirror Support

Configure private package mirrors instead of public repositories.

```toml
# ~/.jarvy/config.toml (global config)
[mirrors]
# Override default package sources
homebrew = "https://mirrors.company.com/homebrew/"
apt = "https://mirrors.company.com/apt/"
npm = "https://mirrors.company.com/npm/"
pypi = "https://mirrors.company.com/pypi/"

# Mirror for Jarvy's own resources (templates, etc.)
jarvy = "https://mirrors.company.com/jarvy/"

[mirrors.custom]
# Custom mirror for specific tools
rust = "https://mirrors.company.com/rustup/"
go = "https://mirrors.company.com/golang/"
```

```bash
# View configured mirrors
jarvy config mirrors

# Output:
# Configured Mirrors
# ==================
#
# homebrew: https://mirrors.company.com/homebrew/
#           Status: ✓ Reachable (latency: 5ms)
#
# apt: https://mirrors.company.com/apt/
#      Status: ✓ Reachable (latency: 8ms)
#
# Default mirrors (no override):
#   dnf, pacman, winget, scoop

# Test mirror connectivity
jarvy config mirrors --test

# Set mirror via CLI
jarvy config set mirrors.homebrew "https://internal/homebrew/"

# Remove mirror (use default)
jarvy config unset mirrors.homebrew
```

**Mirror features:**
- Per-package-manager mirror configuration
- Mirror health checking
- Automatic fallback to default (configurable)
- Support for authenticated mirrors
- Mirror validation before use

### 2. Bundle Mode

Pre-download packages for offline installation.

```bash
# Create bundle for a jarvy.toml
jarvy bundle create

# Output:
# Creating offline bundle...
#
# Analyzing jarvy.toml...
#   Tools to bundle: 8
#   Platform: darwin-arm64
#
# Downloading packages...
#   [1/8] git 2.43.0... 15.2 MB ✓
#   [2/8] node 20.11.0... 45.8 MB ✓
#   [3/8] docker 24.0.7... 520.1 MB ✓
#   [4/8] jq 1.7.1... 1.2 MB ✓
#   [5/8] ripgrep 14.0.0... 3.8 MB ✓
#   [6/8] fd 9.0.0... 2.1 MB ✓
#   [7/8] bat 0.24.0... 4.5 MB ✓
#   [8/8] starship 1.17.0... 5.2 MB ✓
#
# Verifying checksums... ✓
# Creating archive...
#
# ✓ Bundle created: jarvy-bundle-darwin-arm64-20240115.tar.gz
#   Size: 598.2 MB
#   Tools: 8
#   Platform: darwin-arm64
#   Checksum: sha256:abc123...

# Create bundle for specific platform
jarvy bundle create --platform linux-x64

# Create bundle for multiple platforms
jarvy bundle create --platform darwin-arm64,linux-x64,windows-x64

# Create bundle with specific config
jarvy bundle create --config path/to/jarvy.toml

# Create bundle with specific tools only
jarvy bundle create --tools git,node,docker

# List bundle contents
jarvy bundle list jarvy-bundle-darwin-arm64.tar.gz

# Output:
# Bundle Contents
# ===============
#
# Platform: darwin-arm64
# Created: 2024-01-15T10:30:00Z
# Jarvy version: 0.1.0
#
# Tools:
#   git 2.43.0 (15.2 MB)
#   node 20.11.0 (45.8 MB)
#   docker 24.0.7 (520.1 MB)
#   jq 1.7.1 (1.2 MB)
#   ripgrep 14.0.0 (3.8 MB)
#   fd 9.0.0 (2.1 MB)
#   bat 0.24.0 (4.5 MB)
#   starship 1.17.0 (5.2 MB)
#
# Total: 598.2 MB

# Install from bundle
jarvy setup --bundle jarvy-bundle-darwin-arm64.tar.gz

# Output:
# Installing from offline bundle...
#
# Extracting bundle...
# Verifying checksums...
#
# Installing tools...
#   [1/8] git 2.43.0... ✓
#   [2/8] node 20.11.0... ✓
#   ...
#
# ✓ Setup complete (offline mode)
```

**Bundle format:**

```
jarvy-bundle-darwin-arm64-20240115.tar.gz
├── manifest.json           # Bundle metadata
├── jarvy.toml              # Original config
├── packages/
│   ├── git-2.43.0.tar.gz
│   ├── node-20.11.0.tar.gz
│   ├── docker-24.0.7.dmg
│   └── ...
├── checksums.sha256        # Integrity verification
└── install-scripts/        # Platform-specific installers
    ├── git.sh
    ├── node.sh
    └── ...
```

**Bundle features:**
- Single archive for easy transfer
- Platform-specific bundles
- Checksum verification
- Includes install scripts
- Handles tool dependencies

### 3. Offline Validation

Validate configs without network access.

```bash
# Validate with offline mode
jarvy validate --offline

# Output:
# Validating jarvy.toml (offline mode)...
#
# ✓ TOML syntax valid
# ✓ All tool names recognized
# ✓ Version formats valid
# ✓ Hook references valid
#
# ⚠ Remote config validation skipped (offline mode)
#   extends: https://company.com/base.toml
#   Use --online or cache the config first
#
# Validation passed (offline mode)

# Cache remote configs for offline use
jarvy config cache https://company.com/base.toml

# Output:
# Caching remote config...
#   URL: https://company.com/base.toml
#   Cached to: ~/.jarvy/cache/configs/company.com_base.toml
#   Expires: 2024-01-22 (7 days)
#   Checksum: sha256:def456...
#
# Now available for offline use

# List cached configs
jarvy config cache --list

# Output:
# Cached Configs
# ==============
#
# company.com/base.toml
#   Cached: 2024-01-15
#   Expires: 2024-01-22
#   Size: 2.4 KB
#
# company.com/frontend.toml
#   Cached: 2024-01-14
#   Expires: 2024-01-21
#   Size: 1.8 KB

# Refresh cache
jarvy config cache --refresh

# Clear cache
jarvy config cache --clear
```

**Offline validation features:**
- Full TOML validation offline
- Tool name validation (built-in registry)
- Version format validation
- Cached remote config support
- Clear messaging about limitations

### 4. Cache Warming

Pre-populate cache for team deployments.

```bash
# Warm cache for a config
jarvy cache warm

# Output:
# Warming cache for jarvy.toml...
#
# Remote configs:
#   ✓ https://company.com/base.toml (cached)
#   ✓ https://company.com/frontend.toml (cached)
#
# Tool metadata:
#   ✓ git - latest version info cached
#   ✓ node - latest version info cached
#   ✓ docker - latest version info cached
#   ...
#
# Templates:
#   ✓ react template cached
#   ✓ frontend template cached
#
# Cache warmed successfully
#   Total size: 45.2 KB
#   Valid for: 7 days

# Warm cache for specific tools
jarvy cache warm --tools git,node,docker

# Export cache for distribution
jarvy cache export --output jarvy-cache.tar.gz

# Output:
# Exporting cache...
#   Configs: 4 files (8.2 KB)
#   Metadata: 15 files (32.1 KB)
#   Templates: 10 files (5.1 KB)
#
# ✓ Exported to jarvy-cache.tar.gz (45.4 KB)
#
# Import on another machine:
#   jarvy cache import jarvy-cache.tar.gz

# Import cache
jarvy cache import jarvy-cache.tar.gz

# View cache status
jarvy cache status

# Output:
# Cache Status
# ============
#
# Location: ~/.jarvy/cache/
# Total size: 156.3 MB
#
# Categories:
#   Configs: 8 files (12.4 KB)
#   Metadata: 45 files (89.2 KB)
#   Packages: 12 files (156.1 MB)
#   Templates: 10 files (5.1 KB)
#
# Expiration:
#   Expired: 3 items
#   Expiring soon: 5 items
#   Fresh: 67 items

# Clean expired cache
jarvy cache clean
```

**Cache warming features:**
- Pre-fetch all required data
- Exportable cache bundles
- Team cache distribution
- Expiration management
- Selective warming

### 5. Portable Mode

Self-contained Jarvy with embedded tools.

```bash
# Create portable installation
jarvy portable create

# Output:
# Creating portable Jarvy installation...
#
# Base:
#   ✓ Jarvy binary (12.4 MB)
#   ✓ Tool registry (156 KB)
#   ✓ Templates (1.2 MB)
#
# Bundled tools from jarvy.toml:
#   ✓ git 2.43.0 (15.2 MB)
#   ✓ node 20.11.0 (45.8 MB)
#   ✓ jq 1.7.1 (1.2 MB)
#   ...
#
# Creating self-extracting archive...
#
# ✓ Created: jarvy-portable-darwin-arm64.run
#   Size: 245.8 MB
#   Tools: 8
#   Self-contained: Yes
#
# Usage:
#   ./jarvy-portable-darwin-arm64.run
#   # Extracts to ./jarvy-portable/
#   # Run: ./jarvy-portable/bin/jarvy

# Create for specific directory
jarvy portable create --output /path/to/usb/jarvy

# Create minimal (just Jarvy, no tools)
jarvy portable create --minimal

# Update portable installation
jarvy portable update /path/to/jarvy-portable/

# Verify portable installation
jarvy portable verify /path/to/jarvy-portable/
```

**Portable structure:**

```
jarvy-portable/
├── bin/
│   ├── jarvy             # Main binary
│   └── tools/            # Bundled tool binaries
│       ├── git
│       ├── node
│       └── ...
├── lib/                  # Shared libraries
├── data/
│   ├── registry.json     # Tool registry
│   ├── templates/        # Config templates
│   └── cache/            # Pre-warmed cache
├── config/
│   └── config.toml       # Portable config
└── activate.sh           # Environment setup script
```

**Portable features:**
- Self-extracting archive
- No system installation required
- Includes commonly used tools
- USB/removable media friendly
- Environment activation script

## Acceptance Criteria

### Local Mirror Support
- [ ] Mirror configuration in `~/.jarvy/config.toml`
- [ ] Per-package-manager mirror settings
- [ ] Mirror connectivity testing
- [ ] Fallback to default mirrors (configurable)
- [ ] Authenticated mirror support
- [ ] `jarvy config mirrors` displays configuration
- [ ] `jarvy config mirrors --test` validates mirrors

### Bundle Mode
- [ ] `jarvy bundle create` downloads all packages
- [ ] Platform-specific bundle creation
- [ ] Multi-platform bundle support
- [ ] Checksum verification on creation
- [ ] `jarvy bundle list` shows contents
- [ ] `jarvy setup --bundle` installs from bundle
- [ ] Bundle includes install scripts
- [ ] Handles tool dependencies correctly

### Offline Validation
- [ ] `jarvy validate --offline` works without network
- [ ] TOML syntax validation offline
- [ ] Tool name validation offline (built-in registry)
- [ ] Version format validation offline
- [ ] `jarvy config cache` pre-fetches remote configs
- [ ] Cached configs used in offline mode
- [ ] Clear messaging about offline limitations

### Cache Warming
- [ ] `jarvy cache warm` pre-fetches all data
- [ ] `jarvy cache export` creates distributable archive
- [ ] `jarvy cache import` restores from archive
- [ ] `jarvy cache status` shows cache state
- [ ] `jarvy cache clean` removes expired items
- [ ] Selective warming with `--tools` flag
- [ ] Team cache distribution workflow

### Portable Mode
- [ ] `jarvy portable create` builds self-contained install
- [ ] Self-extracting archive format
- [ ] Includes Jarvy binary and tools
- [ ] No system installation required
- [ ] Works from removable media
- [ ] `jarvy portable verify` checks integrity
- [ ] Activation script sets up environment

## Technical Approach

### Module Structure

```
src/
  offline/
    mod.rs              # Offline mode coordination
    mirrors.rs          # Mirror configuration
    bundle.rs           # Bundle creation/extraction
    cache.rs            # Cache management
    portable.rs         # Portable mode
  network/
    mod.rs              # Network abstraction
    offline_check.rs    # Offline detection
    fallback.rs         # Fallback handling
```

### Bundle Creation

```rust
// src/offline/bundle.rs
use std::fs::File;
use tar::Builder;
use flate2::write::GzEncoder;

pub struct BundleCreator {
    config: Config,
    platform: Platform,
    packages: Vec<Package>,
}

impl BundleCreator {
    pub fn create(&self, output: &Path) -> Result<BundleManifest, Error> {
        // Create temp directory for bundle contents
        let temp_dir = tempfile::tempdir()?;

        // Download all packages
        for tool in &self.config.tools {
            let package = self.download_package(tool)?;
            self.packages.push(package);
        }

        // Generate manifest
        let manifest = self.generate_manifest()?;

        // Create install scripts
        self.create_install_scripts(&temp_dir)?;

        // Create checksum file
        self.create_checksums(&temp_dir)?;

        // Build tar.gz archive
        let file = File::create(output)?;
        let enc = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(enc);
        tar.append_dir_all(".", &temp_dir)?;
        tar.finish()?;

        Ok(manifest)
    }

    fn download_package(&self, tool: &Tool) -> Result<Package, Error> {
        let url = self.get_package_url(tool)?;
        let path = self.download_to_temp(&url)?;
        let checksum = self.compute_checksum(&path)?;

        Ok(Package {
            name: tool.name.clone(),
            version: tool.version.clone(),
            path,
            checksum,
        })
    }
}
```

### Offline Detection

```rust
// src/network/offline_check.rs
use std::time::Duration;

pub fn is_offline() -> bool {
    // Check if explicitly set
    if std::env::var("JARVY_OFFLINE").is_ok() {
        return true;
    }

    // Try to reach known endpoints
    let endpoints = [
        "https://api.github.com",
        "https://registry.npmjs.org",
        "https://jarvy.dev/health",
    ];

    let client = ureq::agent();

    for endpoint in endpoints {
        match client
            .get(endpoint)
            .timeout(Duration::from_secs(2))
            .call()
        {
            Ok(_) => return false,  // Online
            Err(_) => continue,
        }
    }

    true  // All endpoints unreachable
}

pub fn require_online(operation: &str) -> Result<(), Error> {
    if is_offline() {
        Err(Error::OfflineMode(format!(
            "Cannot {} in offline mode. Connect to network or use cached data.",
            operation
        )))
    } else {
        Ok(())
    }
}
```

### Mirror Resolution

```rust
// src/offline/mirrors.rs
pub struct MirrorResolver {
    config: MirrorConfig,
}

impl MirrorResolver {
    pub fn resolve_url(&self, source: PackageSource, path: &str) -> String {
        match source {
            PackageSource::Homebrew => {
                if let Some(mirror) = &self.config.homebrew {
                    format!("{}{}", mirror, path)
                } else {
                    format!("https://formulae.brew.sh{}", path)
                }
            }
            PackageSource::Apt => {
                if let Some(mirror) = &self.config.apt {
                    format!("{}{}", mirror, path)
                } else {
                    format!("https://archive.ubuntu.com{}", path)
                }
            }
            // ... other sources
        }
    }

    pub fn test_mirror(&self, source: PackageSource) -> Result<MirrorStatus, Error> {
        let test_url = self.resolve_url(source, "/health");
        let start = Instant::now();

        match ureq::get(&test_url).call() {
            Ok(_) => Ok(MirrorStatus {
                reachable: true,
                latency: start.elapsed(),
            }),
            Err(e) => Ok(MirrorStatus {
                reachable: false,
                error: Some(e.to_string()),
            }),
        }
    }
}
```

## Implementation Steps

1. Create offline module structure
2. Implement mirror configuration
3. Add mirror resolution to package downloads
4. Implement bundle creation
5. Implement bundle extraction and install
6. Add offline validation mode
7. Implement cache warming
8. Add cache export/import
9. Implement portable mode creation
10. Add self-extracting archive support
11. Write offline detection logic
12. Add graceful fallback handling
13. Write unit tests
14. Write integration tests
15. Update documentation

## Dependencies

- `tar` - Archive creation/extraction
- `flate2` - Gzip compression
- `sha2` - Checksum computation
- No new external service dependencies

## Effort Estimate

| Task | Effort |
|------|--------|
| Offline module structure | 0.5 days |
| Mirror configuration | 1.5 days |
| Mirror resolution | 1 day |
| Bundle creation | 3 days |
| Bundle installation | 2 days |
| Offline validation | 1 day |
| Cache warming | 1.5 days |
| Cache export/import | 1 day |
| Portable mode | 3 days |
| Offline detection | 0.5 days |
| Fallback handling | 1 day |
| Testing | 3 days |
| Documentation | 1 day |
| **Total** | **20 days** |

## Files to Create/Modify

### New Files
- `src/offline/mod.rs`
- `src/offline/mirrors.rs`
- `src/offline/bundle.rs`
- `src/offline/cache.rs`
- `src/offline/portable.rs`
- `src/network/mod.rs`
- `src/network/offline_check.rs`
- `src/network/fallback.rs`
- `tests/offline_integration.rs`
- `tests/bundle_integration.rs`
- `tests/portable_integration.rs`

### Modified Files
- `src/main.rs` - Add bundle, cache, portable commands
- `src/config.rs` - Add mirror configuration
- `Cargo.toml` - Add tar, flate2, sha2
- `CLAUDE.md` - Document offline features

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Air-gapped support | None | Full |
| Pre-download for offline | None | Bundle mode |
| Offline validation | Network required | Full offline |
| Mirror support | None | Per-manager |
| Portable deployment | None | Self-contained |
| Cache distribution | None | Export/import |

## Risks

1. **Package format variations**: Different tools package differently
   - Mitigation: Abstract package handling, tool-specific downloaders

2. **Large bundle sizes**: Bundles can be very large
   - Mitigation: Selective bundling, compression, delta updates

3. **Version synchronization**: Bundles can become outdated
   - Mitigation: Version tracking, update warnings

4. **Cross-platform bundles**: Different packages per platform
   - Mitigation: Platform-specific bundles, clear labeling

5. **Mirror reliability**: Private mirrors may be misconfigured
   - Mitigation: Health checks, fallback options, clear errors
