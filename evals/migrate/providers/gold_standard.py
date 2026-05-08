"""Gold-standard provider for offline smoke-testing the eval harness.

Returns the hand-curated `expected.jarvy.toml` for each fixture instead of
calling an LLM. Used by `promptfooconfig.smoke.yaml` to verify the
assertion + fixture wiring in CI without an API key.

Each smoke test passes `source: <fixture-name>` as a var; this provider
reads that var from `context['vars']` and returns the matching expected file.

Promptfoo invokes:

    def call_api(prompt: str, options: dict, context: dict) -> dict
"""

from __future__ import annotations

from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def call_api(prompt: str, options: dict, context: dict) -> dict:
    vars_ = (context or {}).get("vars") or {}
    source = vars_.get("source")
    if not source:
        return {
            "output": "",
            "error": (
                "gold_standard provider: no `source` var. Each test must set "
                "vars.source to the fixture directory name (e.g. 'codespaces')."
            ),
        }

    expected = (
        _repo_root()
        / "tests"
        / "migrate"
        / "fixtures"
        / source
        / "expected.jarvy.toml"
    )
    if not expected.exists():
        return {"output": "", "error": f"gold_standard provider: {expected} missing"}

    return {"output": expected.read_text(encoding="utf-8")}
