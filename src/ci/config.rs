//! CI configuration file generation
//!
//! Generates CI workflow/pipeline configuration files for various providers.

use super::CiProvider;
use std::fmt;
use std::io::Write;
use std::path::Path;

/// Error type for CI config generation
#[derive(Debug)]
pub enum CiConfigError {
    /// IO error when writing config file
    IoError(std::io::Error),
    /// Unsupported provider for config generation
    UnsupportedProvider(CiProvider),
}

impl fmt::Display for CiConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::UnsupportedProvider(p) => {
                write!(f, "Config generation not supported for {}", p)
            }
        }
    }
}

impl std::error::Error for CiConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::UnsupportedProvider(_) => None,
        }
    }
}

impl From<std::io::Error> for CiConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// CI config template for a specific provider
pub struct CiConfigTemplate {
    /// The CI provider
    pub provider: CiProvider,
    /// Template content
    pub content: String,
    /// Suggested file path
    pub file_path: &'static str,
    /// Description of the template
    pub description: &'static str,
}

impl CiConfigTemplate {
    /// Returns the template for the given provider
    pub fn for_provider(provider: CiProvider) -> Option<Self> {
        match provider {
            CiProvider::GitHubActions => Some(Self::github_actions()),
            CiProvider::GitLabCi => Some(Self::gitlab_ci()),
            CiProvider::CircleCi => Some(Self::circleci()),
            CiProvider::AzureDevOps => Some(Self::azure_devops()),
            CiProvider::Bitbucket => Some(Self::bitbucket()),
            _ => None,
        }
    }

    fn github_actions() -> Self {
        Self {
            provider: CiProvider::GitHubActions,
            file_path: ".github/workflows/jarvy.yml",
            description: "GitHub Actions workflow for Jarvy setup",
            content: r#"# Jarvy Development Environment Setup
# This workflow provisions development tools using Jarvy

name: Jarvy Setup

on:
  push:
    branches: [main, master]
    paths:
      - 'jarvy.toml'
  pull_request:
    branches: [main, master]
    paths:
      - 'jarvy.toml'
  workflow_dispatch:

jobs:
  setup:
    name: Provision Environment
    runs-on: ubuntu-latest

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Cache Homebrew packages
        uses: actions/cache@v4
        with:
          path: |
            ~/Library/Caches/Homebrew
            /usr/local/Cellar
          key: ${{ runner.os }}-brew-${{ hashFiles('jarvy.toml') }}
          restore-keys: |
            ${{ runner.os }}-brew-

      - name: Install Jarvy
        run: |
          # Install Jarvy (update with actual installation method)
          cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash

      - name: Run Jarvy Setup
        run: jarvy setup --file jarvy.toml

      - name: Verify Installation
        run: jarvy get --format json

  # Optional: Run on multiple platforms
  setup-matrix:
    name: Setup (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    if: github.event_name == 'workflow_dispatch'
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
      fail-fast: false

    steps:
      - uses: actions/checkout@v4

      - name: Install Jarvy
        run: cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash

      - name: Run Jarvy Setup
        run: jarvy setup --file jarvy.toml
"#
            .to_string(),
        }
    }

    fn gitlab_ci() -> Self {
        Self {
            provider: CiProvider::GitLabCi,
            file_path: ".gitlab-ci.yml",
            description: "GitLab CI configuration for Jarvy setup",
            content: r#"# Jarvy Development Environment Setup
# This pipeline provisions development tools using Jarvy

stages:
  - setup
  - verify

variables:
  # Cache configuration
  JARVY_CACHE_DIR: /cache/jarvy

.jarvy-base:
  cache:
    key: jarvy-$CI_COMMIT_REF_SLUG
    paths:
      - /cache/jarvy
      - ~/.local/share/jarvy
    policy: pull-push

jarvy-setup:
  extends: .jarvy-base
  stage: setup
  image: rust:latest
  script:
    - cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash
    - jarvy setup --file jarvy.toml
  rules:
    - changes:
        - jarvy.toml
      when: always
    - when: manual

jarvy-verify:
  extends: .jarvy-base
  stage: verify
  image: rust:latest
  script:
    - cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash
    - jarvy get --format json
  needs:
    - jarvy-setup
  rules:
    - changes:
        - jarvy.toml
      when: always
    - when: manual

# Multi-platform setup (optional)
.setup-template:
  extends: .jarvy-base
  stage: setup
  script:
    - cargo install jarvy
    - jarvy setup --file jarvy.toml

setup-linux:
  extends: .setup-template
  image: rust:latest
  tags:
    - linux
  when: manual

setup-macos:
  extends: .setup-template
  tags:
    - macos
  when: manual
"#
            .to_string(),
        }
    }

    fn circleci() -> Self {
        Self {
            provider: CiProvider::CircleCi,
            file_path: ".circleci/config.yml",
            description: "CircleCI configuration for Jarvy setup",
            content: r#"# Jarvy Development Environment Setup
# This workflow provisions development tools using Jarvy

version: 2.1

orbs:
  rust: circleci/rust@1.6

executors:
  linux:
    docker:
      - image: cimg/rust:1.75
  macos:
    macos:
      xcode: "15.0"

commands:
  install-jarvy:
    description: "Install Jarvy CLI"
    steps:
      - run:
          name: Install Jarvy
          command: |
            cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash

  run-jarvy-setup:
    description: "Run Jarvy setup"
    steps:
      - run:
          name: Run Jarvy Setup
          command: jarvy setup --file jarvy.toml

jobs:
  setup-linux:
    executor: linux
    steps:
      - checkout
      - restore_cache:
          keys:
            - jarvy-linux-{{ checksum "jarvy.toml" }}
            - jarvy-linux-
      - install-jarvy
      - run-jarvy-setup
      - save_cache:
          key: jarvy-linux-{{ checksum "jarvy.toml" }}
          paths:
            - ~/.cargo
            - ~/.local/share/jarvy
      - run:
          name: Verify Installation
          command: jarvy get --format json

  setup-macos:
    executor: macos
    steps:
      - checkout
      - restore_cache:
          keys:
            - jarvy-macos-{{ checksum "jarvy.toml" }}
            - jarvy-macos-
      - install-jarvy
      - run-jarvy-setup
      - save_cache:
          key: jarvy-macos-{{ checksum "jarvy.toml" }}
          paths:
            - ~/.cargo
            - ~/Library/Caches/Homebrew
      - run:
          name: Verify Installation
          command: jarvy get --format json

workflows:
  jarvy-setup:
    jobs:
      - setup-linux:
          filters:
            branches:
              only:
                - main
                - master
      - setup-macos:
          filters:
            branches:
              only:
                - main
                - master
"#
            .to_string(),
        }
    }

    fn azure_devops() -> Self {
        Self {
            provider: CiProvider::AzureDevOps,
            file_path: "azure-pipelines.yml",
            description: "Azure DevOps pipeline for Jarvy setup",
            content: r#"# Jarvy Development Environment Setup
# This pipeline provisions development tools using Jarvy

trigger:
  branches:
    include:
      - main
      - master
  paths:
    include:
      - jarvy.toml

pr:
  branches:
    include:
      - main
      - master
  paths:
    include:
      - jarvy.toml

variables:
  CARGO_HOME: $(Pipeline.Workspace)/.cargo

stages:
  - stage: Setup
    displayName: 'Jarvy Setup'
    jobs:
      - job: Linux
        displayName: 'Linux Setup'
        pool:
          vmImage: 'ubuntu-latest'
        steps:
          - task: Cache@2
            displayName: 'Cache Cargo'
            inputs:
              key: 'cargo | "$(Agent.OS)" | jarvy.toml'
              path: $(CARGO_HOME)
              restoreKeys: |
                cargo | "$(Agent.OS)"

          - script: |
              cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash
            displayName: 'Install Jarvy'

          - script: |
              jarvy setup --file jarvy.toml
            displayName: 'Run Jarvy Setup'

          - script: |
              jarvy get --format json
            displayName: 'Verify Installation'

      - job: macOS
        displayName: 'macOS Setup'
        pool:
          vmImage: 'macos-latest'
        steps:
          - task: Cache@2
            displayName: 'Cache Cargo'
            inputs:
              key: 'cargo | "$(Agent.OS)" | jarvy.toml'
              path: $(CARGO_HOME)
              restoreKeys: |
                cargo | "$(Agent.OS)"

          - script: |
              cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash
            displayName: 'Install Jarvy'

          - script: |
              jarvy setup --file jarvy.toml
            displayName: 'Run Jarvy Setup'

          - script: |
              jarvy get --format json
            displayName: 'Verify Installation'
"#
            .to_string(),
        }
    }

    fn bitbucket() -> Self {
        Self {
            provider: CiProvider::Bitbucket,
            file_path: "bitbucket-pipelines.yml",
            description: "Bitbucket Pipelines configuration for Jarvy setup",
            content: r#"# Jarvy Development Environment Setup
# This pipeline provisions development tools using Jarvy

image: rust:latest

definitions:
  caches:
    cargo: ~/.cargo
    jarvy: ~/.local/share/jarvy

  steps:
    - step: &install-jarvy
        name: Install Jarvy
        script:
          - cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash

    - step: &jarvy-setup
        name: Run Jarvy Setup
        caches:
          - cargo
          - jarvy
        script:
          - cargo install jarvy || curl -fsSL https://get.jarvy.dev | bash
          - jarvy setup --file jarvy.toml
          - jarvy get --format json

pipelines:
  default:
    - step: *jarvy-setup

  branches:
    main:
      - step: *jarvy-setup
    master:
      - step: *jarvy-setup

  pull-requests:
    '**':
      - step: *jarvy-setup

  custom:
    full-setup:
      - step:
          name: Setup (Linux)
          image: rust:latest
          caches:
            - cargo
          script:
            - cargo install jarvy
            - jarvy setup --file jarvy.toml
"#
            .to_string(),
        }
    }

    /// Writes the template to the appropriate file
    pub fn write(&self, base_path: &Path) -> Result<std::path::PathBuf, CiConfigError> {
        let full_path = base_path.join(self.file_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(&full_path)?;
        file.write_all(self.content.as_bytes())?;

        Ok(full_path)
    }
}

/// Generates CI config for the specified provider
pub fn generate_ci_config(
    provider: CiProvider,
    base_path: &Path,
) -> Result<std::path::PathBuf, CiConfigError> {
    let template = CiConfigTemplate::for_provider(provider)
        .ok_or(CiConfigError::UnsupportedProvider(provider))?;

    template.write(base_path)
}

/// Returns a list of providers that support config generation
pub fn supported_providers() -> Vec<CiProvider> {
    vec![
        CiProvider::GitHubActions,
        CiProvider::GitLabCi,
        CiProvider::CircleCi,
        CiProvider::AzureDevOps,
        CiProvider::Bitbucket,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_github_actions_template() {
        let template = CiConfigTemplate::for_provider(CiProvider::GitHubActions);
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.file_path, ".github/workflows/jarvy.yml");
        assert!(template.content.contains("actions/checkout"));
        assert!(template.content.contains("jarvy setup"));
    }

    #[test]
    fn test_gitlab_ci_template() {
        let template = CiConfigTemplate::for_provider(CiProvider::GitLabCi);
        assert!(template.is_some());
        let template = template.unwrap();
        assert_eq!(template.file_path, ".gitlab-ci.yml");
        assert!(template.content.contains("jarvy-setup"));
        assert!(template.content.contains("stages:"));
    }

    #[test]
    fn test_unsupported_provider() {
        let template = CiConfigTemplate::for_provider(CiProvider::Jenkins);
        assert!(template.is_none());
    }

    #[test]
    fn test_write_template() {
        let temp_dir = TempDir::new().unwrap();
        let result = generate_ci_config(CiProvider::GitHubActions, temp_dir.path());
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.ends_with(".github/workflows/jarvy.yml"));
    }

    #[test]
    fn test_supported_providers() {
        let providers = supported_providers();
        assert!(!providers.is_empty());
        assert!(providers.contains(&CiProvider::GitHubActions));
        assert!(providers.contains(&CiProvider::GitLabCi));
    }
}
