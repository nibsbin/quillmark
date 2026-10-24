#!/usr/bin/env bash
input=$(cat)
if command -v jq >/dev/null; then
  cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
else
  cmd=$input
fi
if printf '%s' "$cmd" | grep -Eq '(^|[;&|(`[:space:]])cargo[[:space:]]+(\+[^[:space:]]+[[:space:]]+)?fmt([[:space:]]|$|[;&|)])'; then
  echo "cargo fmt is blocked: the tree is not rustfmt-formatted, so it rewrites lines the change never touched." >&2
  exit 2
fi
