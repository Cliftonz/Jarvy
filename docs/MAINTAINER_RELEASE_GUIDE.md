# Jarvy Release & Distribution - Maintainer Guide

This guide documents all credentials, secrets, and steps required to set up and manage Jarvy's release process and package distribution.

## Overview

Jarvy uses automated CI/CD pipelines to build, release, and distribute to multiple package managers. This requires several API keys and credentials to be configured as GitHub repository secrets.

## Required GitHub Secrets

The following secrets must be configured in the repository settings under **Settings > Secrets and variables > Actions**:

### 1. crates.io Token (`CRATES_IO_TOKEN`)

**Purpose:** Publish Jarvy to crates.io for `cargo install jarvy`

**How to obtain:**
1. Go to https://crates.io/
2. Log in with your GitHub account
3. Click on your avatar → Account Settings
4. Scroll to "API Tokens"
5. Click "New Token"
6. Name: `jarvy-release`
7. Scopes: Select `publish-new` and `publish-update`
8. Copy the token immediately (it won't be shown again)

**Secret name:** `CRATES_IO_TOKEN`

---

### 2. Homebrew Tap Deploy Key (`HOMEBREW_TAP_DEPLOY_KEY`)

**Purpose:** Push updated formula to the Homebrew tap repository

**Prerequisites:**
1. Create a separate repository: `bearbinary/homebrew-tap`
2. Add a `jarvy.rb` formula file to this repo

**How to obtain:**
```bash
# Generate SSH key pair
ssh-keygen -t ed25519 -C "jarvy-homebrew-tap" -f jarvy-homebrew-tap

# This creates:
# - jarvy-homebrew-tap (private key)
# - jarvy-homebrew-tap.pub (public key)
```

1. Go to `bearbinary/homebrew-tap` repository
2. Settings → Deploy keys → Add deploy key
3. Title: `jarvy-release-bot`
4. Key: Paste contents of `jarvy-homebrew-tap.pub`
5. Check "Allow write access"
6. Click "Add key"

**Secret name:** `HOMEBREW_TAP_DEPLOY_KEY`
**Value:** Contents of `jarvy-homebrew-tap` (private key)

---

### 3. AUR Credentials

**Purpose:** Publish to Arch User Repository (AUR)

#### AUR SSH Private Key (`AUR_SSH_PRIVATE_KEY`)

**How to obtain:**
1. Create an AUR account at https://aur.archlinux.org/
2. Generate SSH key:
```bash
ssh-keygen -t ed25519 -C "aur@jarvy" -f aur-jarvy-key
```
3. Add public key to AUR account:
   - Log in to AUR
   - Go to "My Account"
   - Paste contents of `aur-jarvy-key.pub` into SSH Public Key field
   - Click "Update"

**Secret name:** `AUR_SSH_PRIVATE_KEY`
**Value:** Contents of `aur-jarvy-key` (private key)

#### AUR Username (`AUR_USERNAME`)
Your AUR username

#### AUR Email (`AUR_EMAIL`)
Your AUR account email

---

### 4. winget Token (`WINGET_TOKEN`)

**Purpose:** Submit packages to winget-pkgs repository

**How to obtain:**
1. Go to https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Name: `jarvy-winget-release`
4. Scopes: Select `public_repo`
5. Generate and copy the token

**Secret name:** `WINGET_TOKEN`

**Note:** The winget submission creates a PR to microsoft/winget-pkgs that requires manual approval.

---

### 5. Chocolatey API Key (`CHOCOLATEY_API_KEY`)

**Purpose:** Push packages to Chocolatey community repository

**How to obtain:**
1. Create an account at https://community.chocolatey.org/
2. Go to your account page
3. Find "API Keys" section
4. Copy your API key

**Secret name:** `CHOCOLATEY_API_KEY`

**Note:** First-time package submissions require manual review. Subsequent updates are usually auto-approved.

---

## One-Time Setup Steps

### 1. Create Homebrew Tap Repository

```bash
# Create repository
gh repo create bearbinary/homebrew-tap --public

# Clone and initialize
git clone git@github.com:bearbinary/homebrew-tap.git
cd homebrew-tap

# Copy initial formula
cp /path/to/jarvy/dist/homebrew/jarvy.rb .

# Commit
git add jarvy.rb
git commit -m "Initial formula"
git push
```

Users can then install with:
```bash
brew tap bearbinary/tap
brew install jarvy
```

### 2. Create AUR Package

```bash
# Clone AUR package (first time creates it)
git clone ssh://aur@aur.archlinux.org/jarvy-bin.git
cd jarvy-bin

# Copy PKGBUILD
cp /path/to/jarvy/dist/aur/PKGBUILD-bin PKGBUILD

# Update version and checksums
# Then commit and push
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Initial package"
git push
```

### 3. Install Release Tools Locally

```bash
# cargo-release for version management
cargo install cargo-release

# git-cliff for changelog generation
cargo install git-cliff
```

---

## Release Process

### Standard Release

1. **Prepare the release:**
```bash
# Ensure you're on main with clean working directory
git checkout main
git pull
cargo test
cargo clippy
```

2. **Create the release:**
```bash
# For patch release (0.1.0 -> 0.1.1)
cargo release patch --execute

# For minor release (0.1.0 -> 0.2.0)
cargo release minor --execute

# For major release (0.1.0 -> 1.0.0)
cargo release major --execute
```

This will:
- Bump version in Cargo.toml
- Generate CHANGELOG.md using git-cliff
- Create git commit
- Create git tag (v1.2.3)
- Push to GitHub

3. **Monitor the release:**
- Go to GitHub Actions
- Watch the "Build and Release" workflow
- Once complete, edit the draft release and publish

4. **Verify package updates:**
- Check if Homebrew formula PR is created
- Check if crates.io is updated
- Check if AUR is updated

### Pre-release

```bash
# Create beta release
cargo release --tag-name v1.0.0-beta.1 --execute
```

### Manual Version Override

```bash
# Specify exact version
cargo release 1.2.3 --execute
```

---

## Troubleshooting

### crates.io publish failed
- Check if CRATES_IO_TOKEN is valid
- Ensure all dependencies are published
- Check if version already exists

### Homebrew formula update failed
- Verify HOMEBREW_TAP_DEPLOY_KEY has write access
- Check if homebrew-tap repository exists

### AUR update failed
- Verify SSH key is added to AUR account
- Check PKGBUILD syntax with `makepkg --printsrcinfo`

### winget submission failed
- Manual approval may be required
- Check https://github.com/microsoft/winget-pkgs for PR status

### Chocolatey push failed
- First package requires manual approval
- Check https://community.chocolatey.org/packages/jarvy for status

---

## Security Considerations

1. **Rotate secrets periodically** - Regenerate API keys every 6-12 months
2. **Use minimal scopes** - Only grant necessary permissions
3. **Audit access** - Review who has access to repository secrets
4. **Monitor releases** - Watch for unauthorized release attempts

---

## Useful Commands

```bash
# Check current version
cargo pkgid | cut -d'#' -f2

# Generate changelog without releasing
git-cliff -o CHANGELOG.md

# Dry-run release
cargo release patch --dry-run

# List all tags
git tag -l 'v*'

# View release workflow logs
gh run list --workflow=release.yml
```

---

## Contact

For release issues:
- Open an issue: https://github.com/bearbinary/jarvy/issues
- Tag with `release` label
