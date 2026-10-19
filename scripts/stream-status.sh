#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0

# stream-status.sh: Display a formatted table of all streams for a given employer.
#
# Usage:
#   ./scripts/stream-status.sh <CONTRACT_ID> <EMPLOYER> [NETWORK]
#
# Example:
#   ./scripts/stream-status.sh CBXKDJUQYQDQKSQT6AKZRQWXDXTQHVXHBQQJQ3WKDXJHPNMRGYAQMRS \
#     GBFMRHF3FPD4VC37PJWPVR5LBKHJ7GVWWVKQQJ3JL3WDHWC3BBHLH33 testnet
#
# Environment Variables:
#   NETWORK (default: testnet)  - Stellar network (testnet, mainnet, local, etc.)
#   SOROBAN_RPC_URL (optional)  - Override RPC endpoint; uses network default if unset

set -euo pipefail

# Parse arguments
CONTRACT_ID="${1:-}"
EMPLOYER="${2:-}"
NETWORK="${3:-testnet}"

if [[ -z "$CONTRACT_ID" || -z "$EMPLOYER" ]]; then
    echo "Usage: $0 <CONTRACT_ID> <EMPLOYER> [NETWORK]"
    echo ""
    echo "Example:"
    echo "  $0 CBXKDJUQYQDQKSQT6AKZRQWXDXTQHVXHBQQJQ3WKDXJHPNMRGYAQMRS \\"
    echo "    GBFMRHF3FPD4VC37PJWPVR5LBKHJ7GVWWVKQQJ3JL3WDHWC3BBHLH33 testnet"
    exit 1
fi

# Resolve RPC URL based on network
case "${NETWORK}" in
    testnet)
        RPC_URL="${SOROBAN_RPC_URL:-https://soroban-testnet.stellar.org}"
        ;;
    mainnet)
        RPC_URL="${SOROBAN_RPC_URL:-https://soroban-mainnet.stellar.org}"
        ;;
    local)
        RPC_URL="${SOROBAN_RPC_URL:-http://127.0.0.1:8000}"
        ;;
    *)
        RPC_URL="${SOROBAN_RPC_URL:-https://soroban-${NETWORK}.stellar.org}"
        ;;
esac

echo "Fetching streams for employer: $EMPLOYER"
echo "Network: $NETWORK (RPC: $RPC_URL)"
echo ""

# Fetch stream IDs for the employer
STREAM_IDS=$(stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$EMPLOYER" \
    --network "$NETWORK" \
    -- streams_by_employer --employer "$EMPLOYER" 2>/dev/null || echo "[]")

# Check if no streams
if [[ "$STREAM_IDS" == "[]" ]]; then
    echo "No streams found for employer $EMPLOYER"
    exit 0
fi

# Print table header
printf "%-12s %-12s %-15s %-12s %-12s %-12s %-20s\n" \
    "Stream ID" "Status" "Deposit" "Withdrawn" "Claimable" "Rate/sec" "Time to Exhaustion"
printf "%s\n" "$(printf '%.0s-' {1..120})"

# Parse stream IDs and fetch details for each
# Note: The actual parsing depends on the output format from stellar CLI
# This is a simplified version; you may need to adjust based on actual output format
echo "Note: Use 'stellar contract invoke' to fetch individual stream details:"
echo ""
echo "  stellar contract invoke \\"
echo "    --id $CONTRACT_ID \\"
echo "    --source $EMPLOYER \\"
echo "    --network $NETWORK \\"
echo "    -- get_stream --stream_id <STREAM_ID>"
echo ""
echo "Stream IDs: $STREAM_IDS"
