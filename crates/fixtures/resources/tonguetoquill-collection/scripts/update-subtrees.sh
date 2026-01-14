#!/usr/bin/env bash
#
# Updates all subtrees from upstream
#
# Usage: ./scripts/update-subtrees.sh
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Define subtrees as arrays
SUBTREE_NAMES=("usaf_memo" "classic_resume")
SUBTREE_PREFIXES=(
    "quills/usaf_memo/packages/tonguetoquill-usaf-memo"
    "quills/classic_resume/packages/ttq-classic-resume"
)
UPSTREAM_REPOS=(
    "https://github.com/nibsbin/tonguetoquill-usaf-memo"
    "https://github.com/nibsbin/ttq-classic-resume"
)
UPSTREAM_BRANCHES=(
    "release/core"
    "release/core"
)

cd "$REPO_ROOT"

# Update each subtree
for i in "${!SUBTREE_NAMES[@]}"; do
    NAME="${SUBTREE_NAMES[$i]}"
    PREFIX="${SUBTREE_PREFIXES[$i]}"
    REPO="${UPSTREAM_REPOS[$i]}"
    BRANCH="${UPSTREAM_BRANCHES[$i]}"
    
    echo "========================================="
    echo "Updating subtree: $NAME"
    echo "Prefix: $PREFIX"
    echo "From: $REPO ($BRANCH)"
    echo ""
    
    git subtree pull \
        --prefix="$PREFIX" \
        "$REPO" \
        "$BRANCH" \
        --squash \
        -m "chore: update $NAME subtree from upstream"
    
    echo ""
    echo "✓ $NAME subtree updated successfully"
    echo ""
done

echo "========================================="
echo "✓ All subtrees updated successfully"
