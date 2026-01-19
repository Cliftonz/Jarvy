# External Resource Verification Rule

When recommending repository URLs, packages, plugins, or any external resources:

1. **MUST** use WebSearch to verify the repository/package exists before recommending it
2. **MUST** use WebFetch to confirm the URL is accessible and contains what you claim
3. **NEVER** recommend packages based solely on agent output or memory without verification
4. If uncertain whether something exists, **ALWAYS** search first, recommend second

## This applies to:

### Version Control & Repositories
- GitHub/GitLab/Bitbucket repositories and URLs

### JavaScript/TypeScript
- npm/yarn/pnpm packages

### .NET
- NuGet packages
- .NET tools (`dotnet tool`)

### Rust
- Crates (crates.io)
- Cargo plugins

### Go
- Go modules (pkg.go.dev)

### Ruby
- RubyGems
- Bundler dependencies

### Python
- PyPI packages
- Conda packages

### Shell/Scripting
- Bash scripts and dotfiles repositories
- PowerShell Gallery modules
- PowerShell scripts and repositories

### Editors & IDEs
- Neovim plugins (lazy.nvim, packer, vim-plug)
- VS Code extensions (marketplace)
- JetBrains IDE plugins (IntelliJ, GoLand, RustRover, Rider, RubyMine, etc.)
- Emacs packages (MELPA, ELPA)

### Container & Kubernetes
- Helm charts
- Container images (Docker Hub, ghcr.io, quay.io)
- Kubernetes operators

### Any other external dependencies or links