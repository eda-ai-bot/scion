#!/usr/bin/env bash
# PreToolUse hook (Bash): refuse any shell command whose text mentions the golden-hash
# fixture directory — sed, rm, cp, mv, truncate, redirects, anything. Companion to
# protect-golden.sh, which covers the Edit/Write tools. See CLAUDE.md invariant 5.
set -euo pipefail

input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')

case "$cmd" in
  *crates/scion-types/tests/golden/*|*"tests/golden/"*)
    echo "Blocked: command touches crates/scion-types/tests/golden/ (frozen golden-hash fixtures). Bump FORMAT_VERSION and ask before regenerating (UPDATE_GOLDEN=1)." >&2
    exit 2
    ;;
esac
exit 0
