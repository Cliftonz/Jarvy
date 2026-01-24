# PRD-042: Secrets Management Integration

## Overview

Enable Jarvy to integrate with secrets managers (1Password, HashiCorp Vault, AWS Secrets Manager, etc.) to securely provision credentials, API keys, and tokens during environment setup without storing secrets in configuration files.

## Problem Statement

Development environments require secrets that shouldn't be stored in version control:

- API keys for cloud services (AWS, GCP, Azure)
- Database connection strings
- OAuth tokens and credentials
- SSH keys and signing certificates
- Service account tokens for CI/CD

Currently, developers must manually configure these after Jarvy setup, or teams resort to unsafe practices like sharing `.env` files or committing encrypted secrets.

## Evidence

- `.env.example` files in most repos (secrets redacted)
- Teams share secrets via Slack/email (insecure)
- Onboarding includes "ask John for the API keys" steps
- Secrets management is the #1 post-setup manual step
- Inconsistent secrets between team members cause bugs

## Requirements

### Functional Requirements

1. **Provider support**: Integrate with major secrets managers
2. **Secret references**: Reference secrets by path/key in config
3. **Lazy resolution**: Resolve secrets only when needed
4. **Credential caching**: Cache resolved secrets securely with TTL
5. **Environment injection**: Inject secrets as environment variables
6. **File writing**: Write secrets to specific file paths (e.g., `.env`)
7. **Template support**: Support secrets in templated config files

### Non-Functional Requirements

1. **Never log secrets**: Secrets must never appear in logs or output
2. **Secure memory**: Clear secrets from memory when done
3. **Minimal persistence**: Cache encrypted, respect TTLs
4. **Fail closed**: Fail setup if required secrets unavailable
5. **Provider abstraction**: Easy to add new providers

## Non-Goals

- Full secrets manager administration
- Secret rotation (handled by secrets manager)
- Secrets manager installation/setup
- Encryption key management
- Certificate authority operations

## Feature Specifications

### 1. Configuration Syntax

```toml
# jarvy.toml

[secrets]
# Default provider
provider = "1password"

# Provider configuration
[secrets.providers.1password]
account = "team.1password.com"
vault = "Development"

[secrets.providers.vault]
address = "https://vault.example.com:8200"
auth = "token"  # token, kubernetes, aws-iam
namespace = "dev"

[secrets.providers.aws]
region = "us-west-2"
# Uses default AWS credential chain

[secrets.providers.env]
# Special provider: reads from environment variables
# Useful for CI or local development without secrets manager

# Secret mappings
[secrets.env]
# Inject as environment variables
AWS_ACCESS_KEY_ID = { path = "aws/creds/dev", key = "access_key" }
AWS_SECRET_ACCESS_KEY = { path = "aws/creds/dev", key = "secret_key" }
DATABASE_URL = { path = "database/prod", key = "url" }

# 1Password specific syntax
GITHUB_TOKEN = { op = "op://Development/GitHub Token/credential" }
NPM_TOKEN = { op = "op://Development/NPM/token" }

# Fallback to environment variable if provider unavailable
API_KEY = { path = "api/keys", key = "main", fallback_env = "API_KEY" }

[secrets.files]
# Write secrets to files
".env" = { template = ".env.template" }
"config/database.yml" = { path = "database/config", format = "yaml" }
".ssh/service_key" = { path = "ssh/service", mode = "0600" }
```

### 2. Secret Reference Types

```toml
[secrets.env]
# Simple key-value from path
TOKEN = { path = "tokens/api", key = "value" }

# 1Password URI syntax
SECRET = { op = "op://Vault/Item/field" }

# AWS Secrets Manager
DB_PASS = { aws = "prod/database", key = "password" }

# HashiCorp Vault with mount
API_KEY = { vault = "secret/data/api", key = "key" }

# Environment variable (for CI/local dev)
CI_TOKEN = { env = "CI_TOKEN" }

# With fallback chain
OPTIONAL_KEY = {
    path = "keys/optional",
    fallback = { env = "OPTIONAL_KEY" },
    required = false
}
```

### 3. Template Support

```toml
[secrets.files]
# Template file with secret interpolation
".env" = { template = ".env.template" }

# Template content example (.env.template):
# DATABASE_URL=${secrets.database.url}
# API_KEY=${secrets.api.key}
# AWS_ACCESS_KEY_ID=${secrets.aws.access_key_id}
```

### 4. CLI Commands

```bash
# Check secrets configuration
jarvy secrets check

# Output:
# Secrets Configuration
# =====================
# Provider: 1password (configured)
#
# Environment Variables:
#   AWS_ACCESS_KEY_ID     ✓ resolved
#   AWS_SECRET_ACCESS_KEY ✓ resolved
#   DATABASE_URL          ✓ resolved
#   GITHUB_TOKEN          ✗ not found
#
# Files:
#   .env                  ✓ template found
#   config/database.yml   ✓ will create

# Resolve and display secret paths (not values!)
jarvy secrets list

# Clear cached secrets
jarvy secrets clear-cache

# Test provider connectivity
jarvy secrets test-provider 1password
```

## Technical Approach

### Module Structure

```
src/
  secrets/
    mod.rs           # Public API
    config.rs        # Secrets configuration parsing
    provider.rs      # Provider trait and registry
    cache.rs         # Secure secret caching
    resolve.rs       # Secret resolution logic
    inject.rs        # Environment injection
    files.rs         # File writing with templates
    providers/
      mod.rs         # Provider exports
      onepassword.rs # 1Password CLI integration
      vault.rs       # HashiCorp Vault client
      aws.rs         # AWS Secrets Manager
      env.rs         # Environment variable provider
```

### Configuration Types

```rust
// src/secrets/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SecretsConfig {
    /// Default provider name
    pub provider: Option<String>,

    /// Provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Secrets to inject as environment variables
    #[serde(default)]
    pub env: HashMap<String, SecretRef>,

    /// Secrets to write to files
    #[serde(default)]
    pub files: HashMap<String, FileSecret>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ProviderConfig {
    OnePassword(OnePasswordConfig),
    Vault(VaultConfig),
    Aws(AwsSecretsConfig),
    Env(EnvConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OnePasswordConfig {
    pub account: Option<String>,
    pub vault: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VaultConfig {
    pub address: String,
    pub auth: VaultAuth,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VaultAuth {
    Token,
    Kubernetes,
    AwsIam,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AwsSecretsConfig {
    pub region: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EnvConfig {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SecretRef {
    /// 1Password URI syntax
    OnePassword { op: String },

    /// Generic path/key reference
    PathKey {
        path: String,
        key: Option<String>,
        #[serde(default)]
        required: bool,
        fallback: Option<Box<SecretRef>>,
        fallback_env: Option<String>,
    },

    /// AWS Secrets Manager
    Aws { aws: String, key: Option<String> },

    /// HashiCorp Vault
    Vault { vault: String, key: Option<String> },

    /// Environment variable
    Env { env: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileSecret {
    /// Template file to interpolate
    pub template: Option<String>,

    /// Direct secret path
    pub path: Option<String>,

    /// Output format
    pub format: Option<FileFormat>,

    /// File permissions (Unix octal)
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Raw,
    Json,
    Yaml,
    Env,
}
```

### Provider Trait

```rust
// src/secrets/provider.rs
use async_trait::async_trait;
use secrecy::SecretString;

#[async_trait]
pub trait SecretsProvider: Send + Sync {
    /// Provider name for logging/errors
    fn name(&self) -> &str;

    /// Check if provider is available (CLI installed, authenticated)
    async fn is_available(&self) -> bool;

    /// Resolve a secret by path
    async fn get_secret(&self, path: &str, key: Option<&str>) -> Result<SecretString, SecretsError>;

    /// Check if a secret exists without retrieving it
    async fn secret_exists(&self, path: &str) -> Result<bool, SecretsError>;
}

/// Secure secret value wrapper
pub struct ResolvedSecret {
    pub value: SecretString,
    pub expires_at: Option<std::time::Instant>,
}
```

### 1Password Provider

```rust
// src/secrets/providers/onepassword.rs
use std::process::Command;
use secrecy::SecretString;

pub struct OnePasswordProvider {
    config: OnePasswordConfig,
}

#[async_trait]
impl SecretsProvider for OnePasswordProvider {
    fn name(&self) -> &str {
        "1password"
    }

    async fn is_available(&self) -> bool {
        // Check if op CLI is installed and signed in
        Command::new("op")
            .args(["account", "list"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn get_secret(&self, path: &str, key: Option<&str>) -> Result<SecretString, SecretsError> {
        // Parse 1Password URI or construct from path/key
        let uri = if path.starts_with("op://") {
            path.to_string()
        } else {
            let vault = self.config.vault.as_deref().unwrap_or("Private");
            match key {
                Some(k) => format!("op://{}/{}/{}", vault, path, k),
                None => format!("op://{}/{}", vault, path),
            }
        };

        let output = Command::new("op")
            .args(["read", &uri])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SecretsError::NotFound(format!("{}: {}", uri, stderr)));
        }

        let value = String::from_utf8(output.stdout)?
            .trim()
            .to_string();

        Ok(SecretString::new(value))
    }

    async fn secret_exists(&self, path: &str) -> Result<bool, SecretsError> {
        self.get_secret(path, None).await.map(|_| true).or_else(|e| {
            match e {
                SecretsError::NotFound(_) => Ok(false),
                other => Err(other),
            }
        })
    }
}
```

### Secret Resolution

```rust
// src/secrets/resolve.rs
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;

pub struct SecretsResolver {
    providers: HashMap<String, Box<dyn SecretsProvider>>,
    default_provider: Option<String>,
    cache: SecretsCache,
}

impl SecretsResolver {
    pub async fn resolve_env_secrets(
        &self,
        config: &HashMap<String, SecretRef>,
    ) -> Result<HashMap<String, SecretString>, SecretsError> {
        let mut resolved = HashMap::new();

        for (name, secret_ref) in config {
            match self.resolve_secret(secret_ref).await {
                Ok(secret) => {
                    resolved.insert(name.clone(), secret);
                }
                Err(e) if !secret_ref.is_required() => {
                    eprintln!("  Warning: Optional secret {} not resolved: {}", name, e);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(resolved)
    }

    async fn resolve_secret(&self, secret_ref: &SecretRef) -> Result<SecretString, SecretsError> {
        // Check cache first
        if let Some(cached) = self.cache.get(secret_ref) {
            return Ok(cached);
        }

        let result = match secret_ref {
            SecretRef::OnePassword { op } => {
                let provider = self.get_provider("1password")?;
                provider.get_secret(op, None).await
            }
            SecretRef::PathKey { path, key, fallback, fallback_env, .. } => {
                let provider = self.get_default_provider()?;
                match provider.get_secret(path, key.as_deref()).await {
                    Ok(secret) => Ok(secret),
                    Err(_) if fallback.is_some() => {
                        self.resolve_secret(fallback.as_ref().unwrap()).await
                    }
                    Err(_) if fallback_env.is_some() => {
                        std::env::var(fallback_env.as_ref().unwrap())
                            .map(SecretString::new)
                            .map_err(|_| SecretsError::NotFound(path.clone()))
                    }
                    Err(e) => Err(e),
                }
            }
            SecretRef::Env { env } => {
                std::env::var(env)
                    .map(SecretString::new)
                    .map_err(|_| SecretsError::NotFound(env.clone()))
            }
            SecretRef::Aws { aws, key } => {
                let provider = self.get_provider("aws")?;
                provider.get_secret(aws, key.as_deref()).await
            }
            SecretRef::Vault { vault, key } => {
                let provider = self.get_provider("vault")?;
                provider.get_secret(vault, key.as_deref()).await
            }
        };

        // Cache successful results
        if let Ok(ref secret) = result {
            self.cache.set(secret_ref, secret.clone());
        }

        result
    }
}
```

### Secure Caching

```rust
// src/secrets/cache.rs
use secrecy::SecretString;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct SecretsCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    default_ttl: Duration,
}

struct CacheEntry {
    value: SecretString,
    expires_at: Instant,
}

impl SecretsCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            default_ttl: ttl,
        }
    }

    pub fn get(&self, key: &SecretRef) -> Option<SecretString> {
        let cache_key = Self::make_key(key);
        let entries = self.entries.read().ok()?;

        entries.get(&cache_key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&self, key: &SecretRef, value: SecretString) {
        let cache_key = Self::make_key(key);
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.default_ttl,
        };

        if let Ok(mut entries) = self.entries.write() {
            entries.insert(cache_key, entry);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }
}

impl Drop for SecretsCache {
    fn drop(&mut self) {
        // Clear all secrets from memory
        self.clear();
    }
}
```

## Implementation Steps

1. Create secrets module structure
2. Implement SecretsConfig parsing
3. Implement Provider trait and registry
4. Implement 1Password provider (op CLI)
5. Implement environment variable provider
6. Implement secret resolution with fallbacks
7. Implement secure caching with TTL
8. Implement environment variable injection
9. Implement file writing with templates
10. Add HashiCorp Vault provider
11. Add AWS Secrets Manager provider
12. Integrate with setup command
13. Add CLI commands (check, list, clear-cache)
14. Write tests (with mock providers)
15. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Manual secret configuration | 100% | <10% |
| Secrets shared via insecure channels | Common | Rare |
| Onboarding secrets setup time | 30+ minutes | 2 minutes |
| Secret-related environment bugs | Weekly | Monthly |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Provider CLI not installed | High | Medium | Clear error messages, link to install docs |
| Provider authentication expired | Medium | Medium | Detect and prompt re-authentication |
| Secret not found | Medium | High | Clear error message with path, fallback support |
| Secret logging | Low | Critical | Use SecretString, audit all logging |
| Cache security | Medium | High | Encrypted cache, short TTLs, clear on exit |
| Provider API changes | Low | Medium | Abstract behind trait, version pin CLIs |

## Dependencies

### New Dependencies
- `secrecy` - Secure string wrapper that prevents accidental logging
- `async-trait` - For async provider trait

### Optional Dependencies (per provider)
- `aws-sdk-secretsmanager` - AWS Secrets Manager (feature-gated)
- `vaultrs` - HashiCorp Vault client (feature-gated)

### External Tools
- 1Password CLI (`op`) for [secrets.providers.1password]
- AWS CLI for AWS provider (uses default credential chain)
- Vault CLI optional for [secrets.providers.vault]

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| Provider trait and registry | 0.5 days |
| 1Password provider | 1 day |
| Environment provider | 0.5 days |
| Secret resolution | 1 day |
| Secure caching | 0.5 days |
| Environment injection | 0.5 days |
| File writing/templates | 1 day |
| Vault provider | 1 day |
| AWS provider | 1 day |
| CLI commands | 0.5 days |
| Setup integration | 0.5 days |
| Testing | 1.5 days |
| Documentation | 0.5 days |
| **Total** | **10 days** |

## Files to Create/Modify

### New Files
- `src/secrets/mod.rs`
- `src/secrets/config.rs`
- `src/secrets/provider.rs`
- `src/secrets/cache.rs`
- `src/secrets/resolve.rs`
- `src/secrets/inject.rs`
- `src/secrets/files.rs`
- `src/secrets/providers/mod.rs`
- `src/secrets/providers/onepassword.rs`
- `src/secrets/providers/vault.rs`
- `src/secrets/providers/aws.rs`
- `src/secrets/providers/env.rs`
- `tests/secrets_integration.rs`

### Modified Files
- `src/config.rs` - Add secrets config parsing
- `src/lib.rs` - Export secrets module
- `src/commands/setup_cmd.rs` - Integrate secrets resolution
- `Cargo.toml` - Add secrecy, async-trait, optional provider deps
- `CLAUDE.md` - Document [secrets] section

---

*PRD-042 v1.0 | Secrets Management Integration | Priority: Medium*
