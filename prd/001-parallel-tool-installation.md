# PRD-001: Parallel Tool Installation

## Overview

Implement concurrent tool installation to reduce setup time by up to 10x.

## Problem Statement

Currently, Jarvy installs tools sequentially in `src/main.rs` (lines 218-283). When provisioning multiple tools, each installation blocks the next. A setup with 10 tools taking 30 seconds each requires 5 minutes total, when it could complete in ~30 seconds with parallelization.

## Evidence

- `tests/tools_parallel_install.rs` demonstrates parallel installation is technically possible
- The test spawns threads for git, docker, and jq concurrently (lines 52-63)
- This capability exists in tests but is never used in the actual CLI

## Requirements

### Functional Requirements

1. **Parallel execution**: Install independent tools concurrently
2. **Configurable concurrency**: Allow users to limit parallel jobs (e.g., `--jobs 4`)
3. **Dependency ordering**: Some tools depend on others (e.g., nvm before node)
4. **Progress reporting**: Show real-time status of each tool installation
5. **Error isolation**: One tool failure should not block others

### Non-Functional Requirements

1. Default to number of CPU cores for parallelism
2. Maintain deterministic output ordering for logs
3. Memory usage should scale linearly with job count
4. Support `--sequential` flag for debugging

## Technical Approach

### Option A: Rayon (Recommended)

```rust
use rayon::prelude::*;

tools.par_iter().for_each(|(id, tool)| {
    match tools::add(&tool.name, &tool.version) {
        Ok(()) => println!("Installed {}", tool.name),
        Err(e) => eprintln!("Failed {}: {:?}", tool.name, e),
    }
});
```

**Pros**: Simple API, work-stealing scheduler, mature crate
**Cons**: New dependency

### Option B: Tokio async

```rust
let handles: Vec<_> = tools.iter().map(|(id, tool)| {
    tokio::spawn(async move {
        tools::add(&tool.name, &tool.version)
    })
}).collect();

futures::future::join_all(handles).await;
```

**Pros**: Already common in Rust ecosystem
**Cons**: Async adds complexity, subprocess calls are sync anyway

### Option C: std::thread with channel

```rust
let (tx, rx) = std::sync::mpsc::channel();
let pool = ThreadPool::new(num_cpus::get());

for tool in tools {
    let tx = tx.clone();
    pool.execute(move || {
        let result = tools::add(&tool.name, &tool.version);
        tx.send((tool.name, result)).unwrap();
    });
}
```

**Pros**: No new dependencies
**Cons**: More boilerplate, manual thread management

## Recommended Approach

Use **Rayon** for simplicity. Add to `Cargo.toml`:

```toml
[dependencies]
rayon = "1.10"
```

## Implementation Steps

1. Add `rayon` dependency
2. Refactor `main.rs` setup loop to use `par_iter()`
3. Add `--jobs` CLI flag with default of `num_cpus::get()`
4. Add `--sequential` flag for debugging
5. Implement progress bar using `indicatif` crate
6. Update error collection to aggregate failures
7. Add integration test for parallel installation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| 10-tool setup time | ~5 min | <1 min |
| Memory overhead | N/A | <50MB additional |
| Test coverage | 0% | 80% |

## Risks

1. **Package manager conflicts**: Some PMs lock during install (apt)
   - Mitigation: Serialize per-PM installations, parallelize across PMs
2. **Output interleaving**: Progress messages may overlap
   - Mitigation: Use structured logging with tool prefixes
3. **Resource exhaustion**: Too many parallel downloads
   - Mitigation: Default to conservative job count (4)

## Dependencies

- None (can be implemented independently)

## Effort Estimate

- Implementation: 2-3 days
- Testing: 1 day
- Documentation: 0.5 days

## Files to Modify

- `Cargo.toml` - Add rayon dependency
- `src/main.rs` - Refactor setup loop
- `src/cli.rs` - Add --jobs and --sequential flags
- `tests/tools_parallel_install.rs` - Expand test coverage
