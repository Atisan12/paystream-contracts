#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
#
# Interactive wizard for creating paystream salary streams.
# Prompts for parameters, validates inputs, and submits the create_stream transaction.
#

set -euo pipefail

RPC_URL="${STELLAR_RPC_URL:-http://localhost:8000/rpc}"
NETWORK="${STELLAR_NETWORK:-testnet}"
PASSPHRASE="${STELLAR_NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}"
CONTRACT_ID="${PAYSTREAM_CONTRACT_ID:-}"

if [[ -z "$CONTRACT_ID" ]]; then
  echo "Error: PAYSTREAM_CONTRACT_ID environment variable is required" >&2
  echo "Set it to the deployed stream contract ID." >&2
  exit 1
fi

if ! command -v stellar >/dev/null 2>&1; then
  echo "Error: stellar CLI not found" >&2
  exit 1
fi

echo "=== PayStream Stream Creation Wizard ==="
echo

# Employee address
read -r -p "Employee address (G...): " EMPLOYEE
if [[ -z "$EMPLOYEE" ]]; then
  echo "Error: employee address is required" >&2
  exit 1
fi

# Token selection
echo
echo "Select token:"
echo "1) XLM (native Stellar Lumens)"
echo "2) Other SAC token address"
read -r -p "Choice [1]: " TOKEN_CHOICE
TOKEN_CHOICE=${TOKEN_CHOICE:-1}

if [[ "$TOKEN_CHOICE" == "1" ]]; then
  TOKEN="CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"
  echo "Using XLM SAC address: $TOKEN"
else
  read -r -p "Token contract address (C...): " TOKEN
  if [[ -z "$TOKEN" ]]; then
    echo "Error: token address is required" >&2
    exit 1
  fi
fi

# Deposit amount
read -r -p "Deposit amount (tokens, e.g., 1000): " DEPOSIT
if [[ -z "$DEPOSIT" ]] || ! [[ "$DEPOSIT" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
  echo "Error: deposit must be a positive number" >&2
  exit 1
fi

# Rate per second
echo
echo "Rate configuration:"
read -r -p "Rate per second: " RATE_PER_SECOND
if [[ -z "$RATE_PER_SECOND" ]] || ! [[ "$RATE_PER_SECOND" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
  echo "Error: rate per second must be a positive number" >&2
  exit 1
fi

# Monthly/annual helpers (informational)
MONTHLY=$(awk -v r="$RATE_PER_SECOND" 'BEGIN { printf "%.2f", r * 30 * 24 * 60 * 60 }' 2>/dev/null || echo "N/A")
ANNUAL=$(awk -v r="$RATE_PER_SECOND" 'BEGIN { printf "%.2f", r * 365 * 24 * 60 * 60 }' 2>/dev/null || echo "N/A")
echo "  ~ Monthly: $MONTHLY tokens"
echo "  ~ Annual: $ANNUAL tokens"

# Stop time
echo
read -r -p "Stop time (Unix timestamp, optional; leave blank for indefinite): " STOP_TIME
STOP_TIME=${STOP_TIME:-0}

# Notes
echo
read -r -p "Notes (max 64 chars, optional): " NOTES
NOTES=${NOTES:-}

# Show summary
echo
echo "=== Summary ==="
echo "Contract: $CONTRACT_ID"
echo "Employee: $EMPLOYEE"
echo "Token:    $TOKEN"
echo "Deposit:  $DEPOSIT"
echo "Rate/s:   $RATE_PER_SECOND"
echo "Stop:     $STOP_TIME"
echo "Notes:    ${NOTES:-<none>}"
echo

read -r -p "Confirm and submit? (y/N): " CONFIRM
if [[ "${CONFIRM,,}" != "y" ]]; then
  echo "Cancelled."
  exit 0
fi

# Build invoke command
ARGS=(create_stream --employer "$STELLAR_ACCOUNT" --employee "$EMPLOYEE" --token_address "$TOKEN" --deposit "$DEPOSIT" --rate_per_second "$RATE_PER_SECOND" --stop_time "$STOP_TIME" --low_balance_threshold "[None]" --notes "[None]")
if [[ -n "$NOTES" ]]; then
  # For simplicity, pass as None in basic form; actual CLI may vary
  ARGS=(create_stream --employer "$STELLAR_ACCOUNT" --employee "$EMPLOYEE" --token_address "$TOKEN" --deposit "$DEPOSIT" --rate_per_second "$RATE_PER_SECOND" --stop_time "$STOP_TIME" --low_balance_threshold "[None]" --notes "[Some(\"$NOTES\")]")
fi

echo
echo "Submitting transaction..."
stellar contract invoke --id "$CONTRACT_ID" --source "$STELLAR_ACCOUNT" --network "$NETWORK" "${ARGS[@]}"
echo
echo "Done."
