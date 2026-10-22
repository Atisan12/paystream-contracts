#!/usr/bin/env bash
# OPS-18: Contract ABI/interface diff check.
#
# Generates a JSON interface spec from the compiled stream contract WASM using
# `stellar contract bindings` and diffs it against the committed baseline at
# contracts/stream/interface.json. Fails with a clear diff when the public
# interface (function signatures or event schemas) has changed.
#
# Usage:
#   scripts/check-interface.sh            # verify against committed baseline
#   scripts/check-interface.sh --update   # regenerate the committed baseline
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACT_DIR="${ROOT_DIR}/contracts/stream"
BASELINE="${CONTRACT_DIR}/interface.json"
WASM="${CONTRACT_DIR}/target/wasm32-unknown-unknown/release/stream_contract.wasm"

UPDATE=0
if [[ "${1:-}" == "--update" ]]; then
  UPDATE=1
fi

if ! command -v stellar >/dev/null 2>&1; then
  echo "error: 'stellar' CLI not found; install the Stellar CLI to run the interface check" >&2
  exit 1
fi

if [[ ! -f "${WASM}" ]]; then
  echo "error: compiled WASM not found at ${WASM}" >&2
  echo "       build the contract first (e.g. 'cargo build --target wasm32-unknown-unknown --release' in ${CONTRACT_DIR})" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

# Generate the interface spec from the compiled WASM.
stellar contract bindings json \
  --wasm "${WASM}" \
  --output "${TMP_DIR}/interface.json" >/dev/null

GENERATED="${TMP_DIR}/interface.json"

if [[ "${UPDATE}" -eq 1 ]]; then
  cp "${GENERATED}" "${BASELINE}"
  echo "Updated interface baseline: ${BASELINE}"
  exit 0
fi

if [[ ! -f "${BASELINE}" ]]; then
  echo "error: interface baseline not found at ${BASELINE}" >&2
  echo "       run 'scripts/check-interface.sh --update' to create it" >&2
  exit 1
fi

if diff -u "${BASELINE}" "${GENERATED}" >"${TMP_DIR}/diff.txt"; then
  echo "Contract interface matches committed baseline."
  exit 0
fi

echo "" >&2
echo "========================================================================" >&2
echo " CONTRACT INTERFACE CHANGED" >&2
echo "========================================================================" >&2
echo "The public interface of the stream contract differs from the committed" >&2
echo "baseline at contracts/stream/interface.json. This is a breaking change" >&2
echo "for clients unless intentional." >&2
echo "" >&2
echo "Diff (committed baseline vs. generated):" >&2
echo "------------------------------------------------------------------------" >&2
cat "${TMP_DIR}/diff.txt" >&2
echo "------------------------------------------------------------------------" >&2
echo "" >&2
echo "If this change is intentional, update the baseline and commit it:" >&2
echo "    scripts/check-interface.sh --update" >&2
echo "    git add contracts/stream/interface.json" >&2
echo "" >&2
exit 1
