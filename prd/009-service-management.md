# PRD-009: Service Management

## Overview

Add support for managing background services (databases, caches, containers) as part of the development environment setup.

## Problem Statement

A complete development environment requires not just CLI tools but also running services:
- PostgreSQL, MySQL, MongoDB databases
- Redis, Memcached caches
- Docker containers for microservices
- Local development servers

Currently, Jarvy only installs tools. Developers must manually start/configure services, preventing full environment automation.

## Evidence

- Competitors (docker-compose, Devenv, Vagrant) all manage services
- Common setup step: "Start PostgreSQL before running tests"
- Database configuration is a top friction point for new team members

## Requirements

### Functional Requirements

1. **Service declaration**: Define services in jarvy.toml
2. **Lifecycle management**: Start, stop, restart services
3. **Dependency ordering**: Start database before app
4. **Health checks**: Verify service is ready
5. **Port configuration**: Expose services on specified ports
6. **Docker integration**: Run services as containers
7. **Native services**: Also support brew services, systemd

### Non-Functional Requirements

1. Services survive terminal close (background/daemon)
2. Graceful shutdown on `jarvy stop`
3. Logs accessible via `jarvy logs <service>`
4. Minimal resource usage when idle

## Proposed Config Syntax

```toml
# jarvy.toml

[tools]
docker = "latest"
node = "20"

[services]
# Simple Docker container
postgres = { image = "postgres:15", port = 5432 }
redis = { image = "redis:7", port = 6379 }

# Detailed configuration
[services.postgres]
image = "postgres:15"
port = 5432
env = { POSTGRES_PASSWORD = "devpass", POSTGRES_DB = "myapp_dev" }
volumes = ["./data/postgres:/var/lib/postgresql/data"]
health_check = "pg_isready -U postgres"
depends_on = []

[services.api]
build = "./services/api"  # Dockerfile location
port = 3000
env = { DATABASE_URL = "postgres://postgres:devpass@localhost:5432/myapp_dev" }
depends_on = ["postgres", "redis"]

# Native service (non-Docker)
[services.nginx]
type = "brew"  # Use brew services on macOS
formula = "nginx"
config = "./nginx.conf"

# Custom command service
[services.webpack]
type = "process"
command = "npm run dev"
working_dir = "./frontend"
```

## Technical Approach

### Service Types

```rust
// src/services/mod.rs
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceConfig {
    Docker(DockerService),
    Brew(BrewService),
    Systemd(SystemdService),
    Process(ProcessService),
}

#[derive(Deserialize)]
pub struct DockerService {
    pub image: Option<String>,
    pub build: Option<PathBuf>,
    pub port: Option<u16>,
    pub ports: Option<Vec<String>>,  // "8080:80" format
    pub env: HashMap<String, String>,
    pub volumes: Vec<String>,
    pub health_check: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Deserialize)]
pub struct BrewService {
    pub formula: String,
    pub config: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct ProcessService {
    pub command: String,
    pub working_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
}
```

### Docker Service Manager

```rust
// src/services/docker.rs
use bollard::Docker;

pub struct DockerServiceManager {
    docker: Docker,
    project_name: String,
}

impl DockerServiceManager {
    pub async fn start(&self, name: &str, config: &DockerService) -> Result<(), ServiceError> {
        let container_name = format!("jarvy_{}_{}", self.project_name, name);

        // Pull image if needed
        if let Some(image) = &config.image {
            self.docker.create_image(
                Some(CreateImageOptions { from_image: image, ..Default::default() }),
                None, None
            ).try_collect::<Vec<_>>().await?;
        }

        // Create container
        let container = self.docker.create_container(
            Some(CreateContainerOptions { name: &container_name, .. }),
            Config {
                image: config.image.clone(),
                env: Some(config.env.iter().map(|(k, v)| format!("{}={}", k, v)).collect()),
                exposed_ports: config.port.map(|p| {
                    let mut ports = HashMap::new();
                    ports.insert(format!("{}/tcp", p), HashMap::new());
                    ports
                }),
                host_config: Some(HostConfig {
                    port_bindings: config.port.map(|p| {
                        let mut bindings = HashMap::new();
                        bindings.insert(
                            format!("{}/tcp", p),
                            Some(vec![PortBinding { host_port: Some(p.to_string()), .. }])
                        );
                        bindings
                    }),
                    binds: Some(config.volumes.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }
        ).await?;

        // Start container
        self.docker.start_container::<String>(&container_name, None).await?;

        // Wait for health check
        if let Some(health_cmd) = &config.health_check {
            self.wait_for_health(&container_name, health_cmd).await?;
        }

        Ok(())
    }

    pub async fn stop(&self, name: &str) -> Result<(), ServiceError> {
        let container_name = format!("jarvy_{}_{}", self.project_name, name);
        self.docker.stop_container(&container_name, Some(StopContainerOptions { t: 10 })).await?;
        Ok(())
    }

    pub async fn logs(&self, name: &str, follow: bool) -> Result<impl Stream<Item = String>, ServiceError> {
        let container_name = format!("jarvy_{}_{}", self.project_name, name);
        let logs = self.docker.logs(
            &container_name,
            Some(LogsOptions { follow, stdout: true, stderr: true, ..Default::default() })
        );
        Ok(logs.map(|l| l.to_string()))
    }

    async fn wait_for_health(&self, container: &str, cmd: &str) -> Result<(), ServiceError> {
        for _ in 0..30 {
            let exec = self.docker.create_exec(container, CreateExecOptions {
                cmd: Some(vec!["sh", "-c", cmd]),
                ..Default::default()
            }).await?;

            let result = self.docker.start_exec(&exec.id, None).await?;
            if let StartExecResults::Attached { .. } = result {
                // Check exit code
                let inspect = self.docker.inspect_exec(&exec.id).await?;
                if inspect.exit_code == Some(0) {
                    return Ok(());
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Err(ServiceError::HealthCheckFailed(container.into()))
    }
}
```

### Dependency Resolution

```rust
// src/services/deps.rs
pub fn resolve_order(services: &HashMap<String, ServiceConfig>) -> Result<Vec<String>, ServiceError> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    for (name, config) in services {
        in_degree.entry(name.clone()).or_insert(0);

        let deps = match config {
            ServiceConfig::Docker(d) => &d.depends_on,
            _ => &vec![],
        };

        for dep in deps {
            graph.entry(dep.clone()).or_default().push(name.clone());
            *in_degree.entry(name.clone()).or_insert(0) += 1;
        }
    }

    // Kahn's algorithm for topological sort
    let mut queue: VecDeque<String> = in_degree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut order = Vec::new();

    while let Some(node) = queue.pop_front() {
        order.push(node.clone());

        if let Some(neighbors) = graph.get(&node) {
            for neighbor in neighbors {
                let deg = in_degree.get_mut(neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    if order.len() != services.len() {
        return Err(ServiceError::CyclicDependency);
    }

    Ok(order)
}
```

## CLI Commands

```bash
# Start all services
jarvy up

# Start specific services
jarvy up postgres redis

# Stop all services
jarvy down

# Restart services
jarvy restart

# View status
jarvy services
# Output:
# NAME      STATUS    PORT    IMAGE
# postgres  running   5432    postgres:15
# redis     running   6379    redis:7
# api       stopped   3000    ./services/api

# View logs
jarvy logs postgres
jarvy logs postgres --follow

# Execute command in service
jarvy exec postgres psql -U postgres
```

## Implementation Steps

1. Add `ServicesConfig` to `src/config.rs`
2. Create `src/services/mod.rs` module structure
3. Implement Docker service manager using `bollard` crate
4. Implement brew services support for macOS
5. Implement process manager for custom commands
6. Add dependency resolution with topological sort
7. Add health check waiting logic
8. Implement CLI commands (up, down, logs, exec)
9. Add state file for tracking running services
10. Write integration tests
11. Update documentation

## State Management

```rust
// ~/.jarvy/services.json
{
    "project": "/path/to/project",
    "services": {
        "postgres": {
            "container_id": "abc123...",
            "status": "running",
            "started_at": "2024-01-15T10:30:00Z",
            "port": 5432
        },
        "redis": {
            "container_id": "def456...",
            "status": "running",
            "started_at": "2024-01-15T10:30:05Z",
            "port": 6379
        }
    }
}
```

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Manual service startup | Required | Automated |
| Full stack setup time | 10+ min | < 2 min |
| Service configuration | External | In jarvy.toml |

## Risks

1. **Docker dependency**: Requires Docker installed
   - Mitigation: Support native services (brew, systemd) as fallback
2. **Resource usage**: Many containers use RAM
   - Mitigation: Recommend minimal dev images, document resource needs
3. **Port conflicts**: Services might conflict with existing
   - Mitigation: Detect conflicts, suggest alternatives
4. **Data persistence**: Volume management complexity
   - Mitigation: Default volumes in project directory

## Dependencies

- `bollard` - Docker API client
- `tokio` - Async runtime (may already be present)

## Effort Estimate

- Config parsing: 0.5 days
- Docker manager: 2 days
- Brew services: 0.5 days
- Process manager: 0.5 days
- Dependency resolution: 0.5 days
- Health checks: 0.5 days
- CLI commands: 1 day
- State management: 0.5 days
- Testing: 1 day
- Documentation: 0.5 days

## Files to Create/Modify

- `src/services/mod.rs` - New module
- `src/services/docker.rs` - Docker manager
- `src/services/brew.rs` - Brew services
- `src/services/process.rs` - Process manager
- `src/services/deps.rs` - Dependency resolution
- `src/services/state.rs` - State tracking
- `src/config.rs` - Add ServicesConfig
- `src/main.rs` - Add up/down/logs commands
- `Cargo.toml` - Add bollard dependency
