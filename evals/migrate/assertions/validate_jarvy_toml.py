"""Custom promptfoo assertion: shell out to `jarvy validate` on the LLM output.

The assertion passes if `jarvy validate` produces "Validation passed:" in
stdout (i.e., zero errors). Warnings are tolerated — migration outputs
intentionally exercise warning paths (e.g., version-manager dependencies
not declared in [provisioner]).

Promptfoo invokes this with:

    def get_assert(output: str, context: dict) -> dict

Return a dict with at least `pass` (bool) and `reason` (str). Optionally `score`.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
from pathlib import Path


def _find_jarvy_binary() -> Path:
    """Locate the jarvy binary built by `cargo build --bin jarvy`."""
    here = Path(__file__).resolve()
    # evals/migrate/assertions/<this> → repo root is 3 parents up
    repo_root = here.parents[3]
    debug_bin = repo_root / "target" / "debug" / "jarvy"
    release_bin = repo_root / "target" / "release" / "jarvy"
    if debug_bin.exists():
        return debug_bin
    if release_bin.exists():
        return release_bin
    raise FileNotFoundError(
        f"jarvy binary not found at {debug_bin} or {release_bin}. "
        "Run `cargo build --bin jarvy` from the repo root before invoking promptfoo."
    )


def get_assert(output: str, context: dict) -> dict:
    """Validate that `output` is a jarvy.toml that `jarvy validate` accepts."""
    output = (output or "").strip()
    if not output:
        return {"pass": False, "score": 0.0, "reason": "empty output"}

    if output.startswith("```") or output.endswith("```"):
        return {
            "pass": False,
            "score": 0.0,
            "reason": "output is wrapped in a markdown fence; the prompt asks for raw TOML",
        }

    try:
        binary = _find_jarvy_binary()
    except FileNotFoundError as e:
        return {"pass": False, "score": 0.0, "reason": str(e)}

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".toml", delete=False, encoding="utf-8"
    ) as fh:
        fh.write(output)
        tmp = fh.name

    try:
        result = subprocess.run(
            [str(binary), "validate", "--file", tmp],
            capture_output=True,
            text=True,
            env={**os.environ, "JARVY_TEST_MODE": "1"},
            timeout=30,
        )
    except subprocess.TimeoutExpired:
        os.unlink(tmp)
        return {"pass": False, "score": 0.0, "reason": "jarvy validate timed out"}

    os.unlink(tmp)

    stdout = result.stdout
    if "Validation passed:" in stdout:
        # Score: 1.0 if no warnings, 0.8 if warnings present (still valid).
        warnings = stdout.count("[WARN]")
        score = 1.0 if warnings == 0 else max(0.6, 1.0 - (0.05 * warnings))
        return {
            "pass": True,
            "score": score,
            "reason": f"jarvy validate accepted output ({warnings} warning(s))",
        }

    return {
        "pass": False,
        "score": 0.0,
        "reason": f"jarvy validate rejected output:\n{stdout}",
    }
