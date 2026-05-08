#!/usr/bin/env bash
# Promptfoo exec provider that shells to `claude -p` using the local
# Claude Code session. Reads the rendered prompt from argv[1], pipes to
# claude via stdin, prints model output to stdout.

set -eu

PROMPT="${1:-}"

if [[ -z "$PROMPT" ]]; then
    echo "claude-code.sh: empty prompt (argv[1])" >&2
    exit 1
fi

MODEL="${PROMPTFOO_CLAUDE_MODEL:-sonnet}"

RAW=$(printf '%s' "$PROMPT" | claude \
    -p \
    --output-format text \
    --model "$MODEL")

# Defensive: strip surrounding ```toml ... ``` fences if present.
# LLMs frequently wrap TOML in code fences despite the prompt forbidding it.
# Stripping in the harness lets the eval grade content quality, not formatting.
python3 -c '
import re, sys
raw = sys.stdin.read()
# Strip a leading ```toml or ``` line and a trailing ``` line, if present.
m = re.match(r"^\s*```(?:toml|TOML)?\s*\n(.*)\n```\s*$", raw, re.DOTALL)
sys.stdout.write(m.group(1) if m else raw)
' <<<"$RAW"
