#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0

# label-good-first-issues.sh: Apply bounty and effort labels to GitHub issues
# based on the bounties.md documentation.
#
# This script parses docs/bounties.md to extract the Beginner-Friendly bounty
# table, then applies the appropriate labels (good-first-issue, bounty, small/medium/large)
# to each issue listed.
#
# Usage:
#   ./scripts/label-good-first-issues.sh [--dry-run]
#
# Options:
#   --dry-run    Print labels that would be applied without making changes

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

BOUNTIES_FILE="docs/bounties.md"
REPO="veracindarella/paystream-contracts"

if [[ ! -f "$BOUNTIES_FILE" ]]; then
    echo "Error: $BOUNTIES_FILE not found"
    exit 1
fi

# Extract bounty table from bounties.md (lines between "## Beginner-Friendly Open Bounties" and next section)
# Table format: | [#87](https://github.com/...) | Title | Category | Small | Description |
ISSUES=$(grep -o 'https://github.com/veracindarella/paystream-contracts/issues/[0-9]*' "$BOUNTIES_FILE" | \
    sed 's|.*/issues/||' | sort -u)

if [[ -z "$ISSUES" ]]; then
    echo "No issues found in $BOUNTIES_FILE"
    exit 0
fi

echo "Processing bounty issues from $BOUNTIES_FILE..."
echo ""

for issue_num in $ISSUES; do
    # Extract issue details from bounties.md
    # Parse the table row for this issue
    issue_line=$(grep "#$issue_num" "$BOUNTIES_FILE" | head -1)
    
    if [[ -z "$issue_line" ]]; then
        continue
    fi
    
    # Extract effort level from the table (3rd or 4th column typically contains effort)
    # Look for "Small", "Medium", "Large" keywords
    effort_label="small"
    if echo "$issue_line" | grep -qi "medium"; then
        effort_label="medium"
    elif echo "$issue_line" | grep -qi "large"; then
        effort_label="large"
    fi
    
    # Labels to apply: good-first-issue, bounty, and effort level
    labels="good-first-issue,bounty,$effort_label"
    
    if [[ "$DRY_RUN" == true ]]; then
        echo "#$issue_num: would add labels: $labels"
    else
        echo "Labeling issue #$issue_num with: $labels"
        gh issue edit "$issue_num" \
            --repo "$REPO" \
            --add-label "good-first-issue" \
            --add-label "bounty" \
            --add-label "$effort_label" 2>/dev/null || \
            echo "  Warning: failed to label #$issue_num (may not exist)"
    fi
done

echo ""
if [[ "$DRY_RUN" == true ]]; then
    echo "Dry-run complete. Run without --dry-run to apply labels."
else
    echo "✓ Bounty labels updated successfully."
fi

