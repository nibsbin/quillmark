#!/usr/bin/env bash
#
# Updates the tonguetoquill-usaf-memo subtree from upstream
#
# Usage: ./scripts/update-usaf-memo-subtree.sh
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SUBTREE_PREFIX="quills/usaf_memo/packages/tonguetoquill-usaf-memo"
UPSTREAM_REPO="https://github.com/nibsbin/tonguetoquill-usaf-memo"
UPSTREAM_BRANCH="release/core"

cd "$REPO_ROOT"

echo "Updating subtree: $SUBTREE_PREFIX"
echo "From: $UPSTREAM_REPO ($UPSTREAM_BRANCH)"
echo ""

git subtree pull \
    --prefix="$SUBTREE_PREFIX" \
    "$UPSTREAM_REPO" \
    "$UPSTREAM_BRANCH" \
    --squash \
    -m "chore: update tonguetoquill-usaf-memo subtree from upstream"

echo ""
echo "✓ Subtree updated successfully"
