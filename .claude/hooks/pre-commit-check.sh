#!/usr/bin/env bash
# PreToolUse hook (Bash): before any `git commit`, run rustfmt and clippy; block the commit
# if either fails. Non-commit commands pass through untouched.
set -uo pipefail

input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')

case "$cmd" in
  *"git commit"*|*"git -c "*"commit"*) ;;
  *) exit 0 ;;
esac

cd "$CLAUDE_PROJECT_DIR" || exit 2

# Prefer the toolchain's cargo-fmt: a stale standalone cargo-fmt on PATH would shadow it.
FMT=$(rustup which cargo-fmt 2>/dev/null || echo cargo-fmt)

if ! "$FMT" --all -- --check >/dev/null 2>&1; then
  echo "Blocked: rustfmt check failed. Run: cargo fmt --all" >&2
  exit 2
fi

if ! out=$(cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1); then
  echo "Blocked: clippy failed." >&2
  printf '%s\n' "$out" | grep -E "^(error|warning)" | head -20 >&2
  exit 2
fi

exit 0
