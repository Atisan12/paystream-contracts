#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# sync-labels.sh — apply .github/labels.yml to the GitHub repository
#
# The labels file is the single source of truth for label names, colours, and
# descriptions. This script pushes it to the repository through the `gh` CLI so
# that filtering and automation stay consistent across the whole tracker.
#
# Usage:
#   ./scripts/sync-labels.sh                # create or update every label
#   ./scripts/sync-labels.sh --dry-run      # print what would change, change nothing
#   ./scripts/sync-labels.sh --check        # validate the file and report drift
#   ./scripts/sync-labels.sh --prune        # also delete labels missing from the file
#
# Options:
#   --repo <owner/repo>   repository to operate on
#                         (default: $GITHUB_REPOSITORY, else the git remote)
#   --file <path>         labels file (default: .github/labels.yml)
#
# Exit codes:
#   0  success (or, in --check mode, no problems found)
#   1  usage error, missing dependency, or --check found a problem

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

LABELS_FILE="$REPO_ROOT/.github/labels.yml"
REPO="${GITHUB_REPOSITORY:-}"
DRY_RUN=false
CHECK=false
PRUNE=false

log() { echo "[sync-labels] $*"; }
warn() { echo "[sync-labels] WARN: $*" >&2; }
fail() { echo "[sync-labels] ERROR: $*" >&2; exit 1; }

usage() {
    sed -n '3,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

# ---------------------------------------------------------------------------
# Arguments
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo)
            [[ $# -ge 2 ]] || fail "--repo needs a value"
            REPO="$2"
            shift 2
            ;;
        --file)
            [[ $# -ge 2 ]] || fail "--file needs a value"
            LABELS_FILE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --check)
            CHECK=true
            shift
            ;;
        --prune)
            PRUNE=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail "unknown argument: $1"
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Dependencies
# ---------------------------------------------------------------------------
command -v gh > /dev/null 2>&1 || fail "the GitHub CLI (gh) is required"
[[ -f "$LABELS_FILE" ]] || fail "labels file not found: $LABELS_FILE"

if [[ -z "$REPO" ]]; then
    REPO=$(git -C "$REPO_ROOT" remote get-url origin 2>/dev/null | sed -E 's#^.*github\.com[:/]+##; s#\.git$##') || true
fi
[[ -n "$REPO" ]] || fail "cannot determine the repository; pass --repo owner/name"
[[ "$REPO" == */* ]] || fail "invalid repository '$REPO' — expected owner/name"

# ---------------------------------------------------------------------------
# Parse the labels file
#
# Each entry is:  <name>: ["<hex>", "<description>"]
# ---------------------------------------------------------------------------
declare -a LABEL_NAMES=()
declare -a LABEL_COLORS=()
declare -a LABEL_DESCRIPTIONS=()

parse_labels() {
    local line name colour description
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^[[:space:]]*\"?([^\":]+)\"?[[:space:]]*:[[:space:]]*\[\"([0-9a-fA-F]{6})\",[[:space:]]*\"(.*)\"\][[:space:]]*$ ]] || continue
        name="${BASH_REMATCH[1]}"
        colour="${BASH_REMATCH[2],,}"
        description="${BASH_REMATCH[3]}"
        LABEL_NAMES+=("$name")
        LABEL_COLORS+=("$colour")
        LABEL_DESCRIPTIONS+=("$description")
    done < "$LABELS_FILE"
}

parse_labels

if [[ ${#LABEL_NAMES[@]} -eq 0 ]]; then
    fail "no labels parsed from $LABELS_FILE"
fi

# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------
validate() {
    local problems=0 i name colour description
    declare -A seen=()
    for i in "${!LABEL_NAMES[@]}"; do
        name="${LABEL_NAMES[$i]}"
        colour="${LABEL_COLORS[$i]}"
        description="${LABEL_DESCRIPTIONS[$i]}"

        trimmed="$name"
        while [[ "$trimmed" == [[:space:]]* ]]; do trimmed="${trimmed#?}"; done
        while [[ "$trimmed" == *[[:space:]] ]]; do trimmed="${trimmed%?}"; done
        if [[ -z "$name" || "$trimmed" != "$name" ]]; then
            echo "invalid label name: '$name'" >&2
            problems=$((problems + 1))
        elif [[ "$name" == *:* || "$name" == *\"* ]]; then
            echo "invalid label name: '$name' (colons and quotes are not allowed)" >&2
            problems=$((problems + 1))
        fi
        if [[ -n "${seen[$name]:-}" ]]; then
            echo "duplicate label name: '$name'" >&2
            problems=$((problems + 1))
        fi
        seen[$name]=1
        if [[ ! "$colour" =~ ^[0-9a-f]{6}$ ]]; then
            echo "invalid colour for '$name': '$colour' (expected 6 hex digits)" >&2
            problems=$((problems + 1))
        fi
        if [[ -z "$description" ]]; then
            echo "missing description for '$name'" >&2
            problems=$((problems + 1))
        fi
    done
    return $problems
}

if ! validate; then
    fail "labels file $LABELS_FILE is invalid"
fi

log "Parsed ${#LABEL_NAMES[@]} labels from $LABELS_FILE"

# ---------------------------------------------------------------------------
# Remote state
# ---------------------------------------------------------------------------
remote_labels_json() {
    gh label list --repo "$REPO" --limit 200 --json name,color,description
}

remote_labels_in_use() {
    # Labels currently applied to any issue or pull request in the repository.
    gh api --paginate "repos/$REPO/issues?state=all&per_page=100" \
        --jq '.[].labels[].name' 2>/dev/null | sort -u
}

# ---------------------------------------------------------------------------
# Check mode
# ---------------------------------------------------------------------------
if [[ "$CHECK" == true ]]; then
    problems=0
    remote_json="$(remote_labels_json)"

    # 1. Every label applied in the tracker must be defined in the file.
    while IFS= read -r used; do
        [[ -n "$used" ]] || continue
        found=false
        for name in "${LABEL_NAMES[@]}"; do
            if [[ "${name,,}" == "${used,,}" ]]; then
                found=true
                break
            fi
        done
        if [[ "$found" == false ]]; then
            echo "label '$used' is in use but missing from $LABELS_FILE" >&2
            problems=$((problems + 1))
        fi
    done < <(remote_labels_in_use)

    # 2. Report drift between the file and the repository. Applying the file is
    #    a release step, so drift is reported but does not fail the check.
    for i in "${!LABEL_NAMES[@]}"; do
        name="${LABEL_NAMES[$i]}"
        colour="${LABEL_COLORS[$i]}"
        description="${LABEL_DESCRIPTIONS[$i]}"
        remote_colour=$(echo "$remote_json" | jq -r --arg n "$name" \
            'map(select(.name | ascii_downcase == ($n | ascii_downcase))) | .[0].color // ""')
        remote_description=$(echo "$remote_json" | jq -r --arg n "$name" \
            'map(select(.name | ascii_downcase == ($n | ascii_downcase))) | .[0].description // ""')
        if [[ -z "$remote_colour" ]]; then
            warn "'$name' is defined in the file but not on $REPO — run $0 to apply it"
        elif [[ "$remote_colour" != "$colour" ]]; then
            warn "'$name' colour on $REPO is #$remote_colour, file says #$colour — run $0 to apply it"
        elif [[ "$remote_description" != "$description" ]]; then
            warn "'$name' description on $REPO differs from the file — run $0 to apply it"
        fi
    done

    if [[ $problems -gt 0 ]]; then
        fail "$problems problem(s) found — $LABELS_FILE is not up to date"
    fi

    log "Labels check passed for $REPO"
    exit 0
fi

# ---------------------------------------------------------------------------
# Apply mode
# ---------------------------------------------------------------------------
remote_json="$(remote_labels_json)"

for i in "${!LABEL_NAMES[@]}"; do
    name="${LABEL_NAMES[$i]}"
    colour="${LABEL_COLORS[$i]}"
    description="${LABEL_DESCRIPTIONS[$i]}"

    remote_colour=$(echo "$remote_json" | jq -r --arg n "$name" \
        'map(select(.name | ascii_downcase == ($n | ascii_downcase))) | .[0].color // ""')
    remote_description=$(echo "$remote_json" | jq -r --arg n "$name" \
        'map(select(.name | ascii_downcase == ($n | ascii_downcase))) | .[0].description // ""')

    if [[ "$remote_colour" == "$colour" && "$remote_description" == "$description" ]]; then
        log "up to date: $name"
        continue
    fi

    if [[ "$DRY_RUN" == true ]]; then
        if [[ -z "$remote_colour" ]]; then
            log "would create: $name (#$colour) — $description"
        else
            log "would update: $name (#$colour) — $description"
        fi
        continue
    fi

    log "creating/updating label: $name (#$colour)"
    gh label create "$name" --repo "$REPO" --color "$colour" --description "$description" --force
done

# ---------------------------------------------------------------------------
# Prune — delete labels on the repository that are not in the file
# ---------------------------------------------------------------------------
if [[ "$PRUNE" == true ]]; then
    while IFS= read -r extra; do
        [[ -n "$extra" ]] || continue
        keep=false
        for name in "${LABEL_NAMES[@]}"; do
            if [[ "${name,,}" == "${extra,,}" ]]; then
                keep=true
                break
            fi
        done
        [[ "$keep" == false ]] || continue
        if [[ "$DRY_RUN" == true ]]; then
            log "would delete: $extra"
        else
            log "deleting label not in file: $extra"
            gh label delete "$extra" --repo "$REPO" --yes
        fi
    done < <(echo "$remote_json" | jq -r '.[].name')
fi

log "Done. Labels of $REPO now match $LABELS_FILE"
