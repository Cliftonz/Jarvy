# Migration prompt evals (promptfoo)

Quality harness for the per-system migration prompts shipped in `docs/migrate/`.

## What this measures

For each migration source (Codespaces, DevPod, Gitpod, Dev Containers, Vagrant,
Brewfile, mise, asdf, Nix):

1. **Mechanical correctness** — the LLM's output passes `jarvy validate`
   without errors (warnings are allowed).
2. **No code fence** — the output is raw TOML, not wrapped in markdown.
3. **Required tools present** — specific known tools from the source fixture
   appear in the output.
4. **Container-only concepts dropped** — `image`, `forwardPorts`, `mounts`, etc.
   do NOT appear.
5. **Intent preservation** — an LLM rubric scores whether the migration
   captures the source's spirit (hooks, env vars, role assignments).

## Inputs

Fixtures live in `tests/migrate/fixtures/<source>/`:

- `input.<source>` — original config (devcontainer.json, Brewfile, etc.)
- `expected.jarvy.toml` — hand-curated gold-standard output (used for
  reference and as a test oracle in the LLM rubric)

These are the same fixtures consumed by the Rust integration test
`tests/migrate_fixtures.rs`.

## Running locally

```bash
# Build the jarvy binary (needed for the validate assertion)
cargo build --bin jarvy

# Install promptfoo (one-time)
npm install -g promptfoo

# Run the eval
cd evals/migrate
export ANTHROPIC_API_KEY=sk-ant-...
promptfoo eval

# View results in the browser
promptfoo view
```

## Running in CI

Add to `.github/workflows/`:

```yaml
- run: cargo build --bin jarvy
- run: npm install -g promptfoo
- working-directory: evals/migrate
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
  run: promptfoo eval --output results.json
```

## Layout

```
evals/migrate/
├── README.md                       # this file
├── promptfooconfig.yaml            # eval definition
├── prompts/                        # one prompt per source
│   ├── codespaces.txt
│   ├── devpod.txt
│   ├── gitpod.txt
│   ├── dev-containers.txt
│   ├── vagrant.txt
│   ├── homebrew-bundle.txt
│   ├── mise.txt
│   ├── asdf.txt
│   └── nix.txt
└── assertions/
    └── validate_jarvy_toml.py      # shells out to `jarvy validate`
```

## Updating the prompts

The `prompts/*.txt` files are the **canonical** versions used by both the eval
harness and the user-facing docs. When you edit a prompt, update both:

1. `evals/migrate/prompts/<source>.txt` — for the eval
2. `docs/migrate/from-<source>.md` — for the user

Re-run `promptfoo eval` to verify the prompt change didn't regress quality.
