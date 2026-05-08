"""Promptfoo provider that shells out to the Claude Code CLI (`claude -p`).

Uses the local Claude Code OAuth session so the eval works without exporting
ANTHROPIC_API_KEY. Set `model` in promptfoo provider config to choose the
Claude variant (e.g. "sonnet", "opus", "claude-sonnet-4-6").

Promptfoo invokes:

    def call_api(prompt: str, options: dict, context: dict) -> dict
"""

from __future__ import annotations

import os
import shutil
import subprocess


def _resolve_claude_binary() -> str:
    binary = shutil.which("claude")
    if not binary:
        raise FileNotFoundError(
            "claude CLI not found on PATH. Install Claude Code or use the "
            "anthropic:messages:* provider with ANTHROPIC_API_KEY set."
        )
    return binary


def call_api(prompt: str, options: dict, context: dict) -> dict:
    cfg = (options or {}).get("config") or {}
    model = cfg.get("model", "sonnet")
    timeout = int(cfg.get("timeout", 180))

    try:
        binary = _resolve_claude_binary()
    except FileNotFoundError as e:
        return {"output": "", "error": str(e)}

    cmd = [
        binary,
        "-p",
        "--output-format", "text",
        "--model", model,
        # Skip auto-features that could pollute the output. We pipe the
        # prompt via stdin instead of as an arg so multiline content is safe.
        "--disable-slash-commands",
        "--no-session-persistence",
    ]

    try:
        proc = subprocess.run(
            cmd,
            input=prompt,
            capture_output=True,
            text=True,
            timeout=timeout,
            env={**os.environ, "CLAUDE_CODE_SIMPLE": "1"},
        )
    except subprocess.TimeoutExpired:
        return {
            "output": "",
            "error": f"claude -p timed out after {timeout}s",
        }
    except Exception as e:  # noqa: BLE001
        return {"output": "", "error": f"claude -p invocation failed: {e}"}

    if proc.returncode != 0:
        return {
            "output": proc.stdout.strip(),
            "error": (
                f"claude -p exited {proc.returncode}\n"
                f"stderr: {proc.stderr.strip()[:2000]}"
            ),
        }

    return {"output": proc.stdout.strip()}
