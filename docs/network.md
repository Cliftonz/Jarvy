---
title: "Network & Proxy - Jarvy"
description: "Run Jarvy behind corporate proxies and custom CA bundles. Per-tool overrides, secure credentials, environment propagation."
---

# Network & Proxy

Jarvy works in restricted corporate networks. The `[network]` section in `jarvy.toml` configures HTTP/HTTPS/SOCKS proxies, optional auth, custom CA bundles, and per-tool overrides. All settings are propagated to child processes (brew, apt, winget, pip, npm, cargo, etc.) via standard env vars.

## Minimal Example

```toml
[network]
https_proxy = "http://proxy.corp.com:8080"
no_proxy = ["localhost", "127.0.0.1", ".corp.com"]
```

Run `jarvy setup` and every package manager invocation inherits `HTTPS_PROXY` + `NO_PROXY`.

## Full Configuration

```toml
[network]
http_proxy = "http://proxy.corp.com:8080"
https_proxy = "http://proxy.corp.com:8080"
all_proxy = "socks5://socks.corp.com:1080"   # Falls back if http/https not set
no_proxy = ["localhost", "127.0.0.1", ".corp.com", "192.168.0.0/16"]

[network.auth]
username = "jdoe"
password = { env = "PROXY_PASSWORD" }   # Read from env at runtime

[network.tls]
ca_bundle = "/etc/ssl/certs/corporate-ca.crt"
insecure = false                         # Disable cert verification (avoid)

# Per-tool overrides
[network.overrides.git]
https_proxy = "http://git-proxy.corp.com:8888"

[network.overrides.npm]
https_proxy = "http://npm-proxy.corp.com:8090"
```

## Resolution Priority

1. Tool-specific override (`[network.overrides.<tool>]`)
2. Global Jarvy network config (`[network]`)
3. Environment variables already set in the user's shell
4. System defaults

Higher precedence wins. Env vars set in the shell are **not** clobbered by lower-priority Jarvy config — your local override stays in effect.

## Credentials

Never put plaintext passwords in `jarvy.toml`. Use env-var indirection:

```toml
[network.auth]
username = "jdoe"
password = { env = "PROXY_PASSWORD" }
```

Or read from a file (mode `0600` recommended):

```toml
[network.auth]
password = { from_file = "~/.secrets/proxy_password" }
```

Jarvy expands `~` and resolves the file at runtime. The file is **not** included in debug tickets or logs.

## Environment Variables Propagated

When Jarvy spawns a child process, it sets:

| Variable | Source |
|----------|--------|
| `HTTP_PROXY`, `http_proxy` | `[network].http_proxy` |
| `HTTPS_PROXY`, `https_proxy` | `[network].https_proxy` |
| `ALL_PROXY`, `all_proxy` | `[network].all_proxy` |
| `NO_PROXY`, `no_proxy` | `[network].no_proxy` (joined with `,`) |
| `CURL_CA_BUNDLE` | `[network.tls].ca_bundle` |
| `SSL_CERT_FILE` | `[network.tls].ca_bundle` |
| `NODE_EXTRA_CA_CERTS` | `[network.tls].ca_bundle` |
| `GIT_SSL_CAINFO` | `[network.tls].ca_bundle` |
| `REQUESTS_CA_BUNDLE` | `[network.tls].ca_bundle` |

Both upper- and lower-case forms are set so legacy tools work.

## Custom CA Bundle

For self-signed corporate roots:

```toml
[network.tls]
ca_bundle = "/etc/ssl/certs/corporate-ca.crt"
```

Jarvy itself uses `rustls` and reads the bundle directly. Child processes get the bundle path via the env vars above. Works with curl, wget, npm, pip, cargo, brew, git, apt, dnf.

## Per-Tool Overrides

Some tools need a different proxy than the default — for example, an internal git mirror reached over a different gateway:

```toml
[network.overrides.git]
https_proxy = "http://git-proxy.corp.com:8888"
no_proxy = ["github.internal"]
```

Override fields fully replace the global value for that tool — they are not merged. Omit a field to fall through to the global.

## Testing Connectivity

```bash
jarvy doctor --extended            # Includes proxy reachability checks
jarvy telemetry test               # Fails fast if OTLP endpoint is unreachable
```

For deeper debugging, run with `-vv` to see the resolved proxy chain:

```bash
jarvy setup -vv 2>&1 | grep -i proxy
```

## Common Pitfalls

- **`no_proxy` matches by suffix**: `.corp.com` matches `git.corp.com` and `corp.com`. Bare `corp.com` matches only the exact string in many implementations — use the leading dot.
- **HTTPS proxy with HTTP scheme**: Most corporate proxies are HTTP CONNECT proxies, even for HTTPS traffic. `https_proxy = "http://..."` is correct.
- **MITM proxies + Cargo**: If your proxy intercepts TLS, set `[network.tls].ca_bundle` so Cargo's rustls trusts the corporate root.

## Module

- Source: `src/network/`
- Key types: `NetworkConfig`, `ProxyAuth`, `TlsConfig`, `NetworkOverride`
- Resolution: `src/network/resolve.rs`
- Propagation: `src/network/propagate.rs`
