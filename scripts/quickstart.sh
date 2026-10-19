#!/usr/bin/env bash
#
# scripts/quickstart.sh — One-command PayStream quickstart on Stellar Testnet.
#
# Generates employer + employee accounts, funds them via the Stellar Friendbot,
# deploys the PayStream token and stream contracts, initialises them, creates a
# first salary stream, then prints the stream ID and claimable amount after
# ~5 seconds of accrual.
#
# Usage: ./scripts/quickstart.sh
#
# Requirements:
#   - Stellar CLI (stellar)
#   - Rust toolchain with wasm32-unknown-unknown target (for `stellar contract build`)
set -euo pipefail

NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
FRIENDBOT_URL="https://friendbot.stellar.org?addr="
DEPOSIT=10000000      # 1 XLM in stroops
RATE=10000            # stroops per second

log() { printf '[quickstart] %s\n' "$*"; }

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        log "ERROR: '$1' is required but not found on PATH."
        exit 1
    fi
}

require_cmd stellar
require_cmd curl

# ---------------------------------------------------------------------------
# 0. Network / target checks
# ---------------------------------------------------------------------------
log "Ensuring testnet network is configured..."
if ! stellar network list | grep -q testnet; then
    stellar network add testnet \
        --rpc-url "$RPC_URL" \
        --network-passphrase "$NETWORK_PASSPHRASE"
fi

log "Building contracts (wasm32-unknown-unknown)..."
stellar contract build

# ---------------------------------------------------------------------------
# 1. Generate + fund employer and employee accounts via Friendbot
# ---------------------------------------------------------------------------
log "Generating employer account..."
stellar keys generate --global quickstart-employer --network "$NETWORK" --fund >/dev/null 2>&1 || {
    stellar keys generate --global quickstart-employer --network "$NETWORK"
    EMPLOYER_ADDR=$(stellar keys address quickstart-employer --network "$NETWORK")
    log "Funding employer via Friendbot: $EMPLOYER_ADDR"
    curl -sf "$FRIENDBOT_URL$EMPLOYER_ADDR" >/dev/null
}
EMPLOYER=$(stellar keys address quickstart-employer --network "$NETWORK")

log "Generating employee account..."
stellar keys generate --global quickstart-employee --network "$NETWORK" --fund >/dev/null 2>&1 || {
    stellar keys generate --global quickstart-employee --network "$NETWORK"
    EMPLOYEE_ADDR=$(stellar keys address quickstart-employee --network "$NETWORK")
    log "Funding employee via Friendbot: $EMPLOYEE_ADDR"
    curl -sf "$FRIENDBOT_URL$EMPLOYEE_ADDR" >/dev/null
}
EMPLOYEE=$(stellar keys address quickstart-employee --network "$NETWORK")

log "Employer: $EMPLOYER"
log "Employee: $EMPLOYEE"

# ---------------------------------------------------------------------------
# 2. Deploy contracts
# ---------------------------------------------------------------------------
log "Deploying PayStream token contract..."
TOKEN_ID=$(stellar contract deploy \
    --wasm target/wasm32-unknown-unknown/release/paystream_token.wasm \
    --source default --network "$NETWORK")
log "Token: $TOKEN_ID"

log "Deploying PayStream stream contract..."
STREAM_ID=$(stellar contract deploy \
    --wasm target/wasm32-unknown-unknown/release/paystream_stream.wasm \
    --source default --network "$NETWORK")
log "Stream: $STREAM_ID"

# ---------------------------------------------------------------------------
# 3. Initialise contracts
# ---------------------------------------------------------------------------
log "Initialising token and stream contracts..."
stellar contract invoke --id "$TOKEN_ID" --source default --network "$NETWORK" \
    -- initialize --admin "$EMPLOYER" --initial_supply 1000000000 >/dev/null
stellar contract invoke --id "$STREAM_ID" --source default --network "$NETWORK" \
    -- initialize --admin "$EMPLOYER" >/dev/null
log "Contracts initialised."

# ---------------------------------------------------------------------------
# 4. Create the first stream (employer funds, employee receives)
# ---------------------------------------------------------------------------
log "Creating first stream (deposit=$DEPOSIT stroops, rate=$RATE stroops/s)..."
STREAM_ID_OUT=$(stellar contract invoke --id "$STREAM_ID" --source "$EMPLOYER" --network "$NETWORK" \
    -- create_stream \
    --employer "$EMPLOYER" \
    --employee "$EMPLOYEE" \
    --token_address "$TOKEN_ID" \
    --deposit "$DEPOSIT" \
    --rate_per_second "$RATE" \
    --stop_time 0)
STREAM_ID_OUT="${STREAM_ID_OUT//\"/}"
log "Stream created with ID: $STREAM_ID_OUT"

# ---------------------------------------------------------------------------
# 5. Wait ~5s for accrual, then show stream ID + claimable amount
# ---------------------------------------------------------------------------
log "Waiting 5 seconds for tokens to accrue..."
sleep 5

CLAIMABLE=$(stellar contract invoke --id "$STREAM_ID" --source "$EMPLOYEE" --network "$NETWORK" \
    -- claimable --stream_id "$STREAM_ID_OUT")
CLAIMABLE="${CLAIMABLE//\"/}"

echo
echo "========================================="
echo "Quickstart complete!"
echo "Stream ID : $STREAM_ID_OUT"
echo "Claimable : $CLAIMABLE stroops (after ~5s)"
echo "Token     : $TOKEN_ID"
echo "Employer  : $EMPLOYER"
echo "Employee  : $EMPLOYEE"
echo "========================================="
