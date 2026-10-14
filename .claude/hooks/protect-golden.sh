#!/usr/bin/env bash
# PreToolUse hook (Edit/Write/MultiEdit/NotebookEdit): refuse edits to the golden-hash
# fixtures. They are the executable form of docs/DATA_MODEL.md; see CLAUDE.md invariant 5.
set -euo pipefail

input=$(cat)
path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty')

case "$path" in
  *crates/scion-types/tests/golden/*)
    echo "Blocked: $path is a frozen golden-hash fixture. Bump FORMAT_VERSION and ask before regenerating (UPDATE_GOLDEN=1)." >&2
    exit 2
    ;;
esac
exit 0
