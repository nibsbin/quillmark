#!/usr/bin/env bash
[ "$CLAUDE_CODE_REMOTE" = "true" ] || exit 0
cd "$CLAUDE_PROJECT_DIR" && cargo fetch --quiet
