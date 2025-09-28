---
name: New package request
about: Request adding a package/tool to official installation docs and tooling
title: '[PKG] Package name'
labels: 'enhancement'
assignees: ''
---

## Package information
- Name:
- Homepage:
- License:
- Summary (1–2 lines):

## Why should this be added?
Describe the use case and value for users/contributors.

## Minimum version (if any)
- Version hint (e.g., ">=1.2", "latest"):

## Installation instructions (required)
Provide exact, copy-pastable commands for each platform/manager.

### macOS (Homebrew preferred)
- Brew formula/cask:
- Install:
    - brew install <formula>
    - or, if GUI app: brew install --cask <cask>
- Verify command:

### Windows (winget)
- Winget ID:
- Install:
    - winget install --id <Publisher.Package> -e
- Verify command:

### Linux

#### Debian/Ubuntu (apt)
- Repo setup (if needed):
- Install:
    - sudo apt-get update
    - sudo apt-get install -y <packages>
- Verify command:

#### Fedora/RHEL (dnf)
- Repo setup (if needed):
- Install:
    - sudo dnf install -y <packages>
- Verify command:

#### RHEL/CentOS (yum)
- Repo setup (if needed):
- Install:
    - sudo yum install -y <packages>
- Verify command:

#### Alpine (apk)
- Repo setup (if needed):
- Install:
    - sudo apk add --no-cache <packages>
- Verify command:

#### Arch (pacman)
- Repo setup (if needed):
- Install:
    - sudo pacman -S --noconfirm <packages>
- Verify command:

#### Other distro/package manager (optional)
- Manager:
- Install:
    - <commands>
- Verify command:

## Post-install steps (if any)
- Environment variables, services, permissions, PATH updates, etc.

## Uninstall instructions
Provide commands for each platform/manager.

## References
- Official installation guide URL(s):
- Checksums/signature verification info (if applicable):

## Testing notes
- How to validate installation works (commands or sample output):
