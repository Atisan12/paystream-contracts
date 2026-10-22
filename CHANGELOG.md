# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_Changes after the v0.2.0 release will be listed here._

## [0.2.0] - Unreleased

### Added

- PROD-17 (#306): cliff vesting — `cliff_time: Option<u64>` on `create_stream` and `StreamParams`, `get_cliff_time`, `cliff_reached` event, E035 `ERR_CLIFF_AFTER_STOP`. **Breaking:** `create_stream` takes an extra trailing argument and `StreamParams` gains a field (saved templates from earlier builds no longer decode).
- PROD-20 (#309): `delete_template` and `get_template`; templates carry `cliff_time`.
- TOK-20 (#305): SEP-41 compliance — `decimals` / `name` / `symbol` with write-once `set_metadata`, `transfer` / `approve` / `burn` / `mint` events, `transfer` accepts a `MuxedAddress`. **Breaking:** `allowance` returns `i128` (0 when expired); the old tuple is `allowance_with_expiration`. See `docs/sep41-compliance.md`.
- TOK-19 (#304): token proptest fuzz targets (`contracts/token/fuzz`) and a CI job.

### Fixed

- Token `transfer_from` accepted negative amounts (moving tokens to `from` and raising the allowance); `approve` accepted negative amounts.
- Stream `update_rate` double-counted previously banked `pending_accrual` on every rate change.
- Stream contract did not compile on `main` (definitions dropped by merges restored from the commits that introduced them; soroban-sdk 28 test API updates).

### Added (earlier)

- Canonical read-only stream index entrypoints `stream_ids_by_employer` and `stream_ids_by_employee`; the former `streams_by_*` entrypoints remain as deprecated aliases.
- SC-06: `stream_status(stream_id) -> StreamStatus` — lightweight view that returns only
  the stream status without loading the full `Stream` struct into the caller's response.
  Reduces resource consumption for read-heavy off-chain indexers polling many streams.
- SC-31: `pause_stream_batch(employer, stream_ids)` — bulk pause entry point allowing employers
  to pause up to 50 active streams atomically in a single transaction. Emits status events on success
  and reverts atomically if any stream in the batch fails.
- SC-32: `resume_stream_batch(employer, stream_ids)` — bulk resume entry point allowing employers
  to resume up to 50 paused streams atomically in a single transaction, restarting accrual and resetting
  last_withdraw_time for each stream. Emits status events on success and reverts atomically if any stream is not Paused.
- SC-34: `get_auto_topup_config(stream_id) -> Option<AutoTopupConfig>` — public read-only query function
  for retrieving recurring auto top-up configuration for off-chain dashboards and indexers.
- `set_low_balance_threshold(employer, stream_id, threshold)` — allows employers to update or disable (`threshold = 0`) the low-balance alert threshold at any time without topping up. Emits `threshold_updated` event.
- `withdraw_all(employee)` — withdraws from every stream the employee holds.
- `get_streams_batch(stream_ids)` and `claimable_batch(stream_ids)` — batch read entry points.
- `update_rate(employer, stream_id, new_rate)` — changes a stream's rate mid-flight; earnings accrued
  at the previous rate are preserved in `pending_accrual`.
- `transfer_stream(employer, stream_id, new_employer)` — reassigns a stream to a new employer.
- `settle_stream(stream_id)` and `expire_streams(admin, stream_ids)` — finalise and expire streams.
- `receipt_owner(stream_id)` and `transfer_receipt(from, to, stream_id)` — transferable stream receipts.
- `save_template`, `create_stream_from_template`, and `list_templates` — reusable stream templates.
- `set_auto_topup`, `cancel_auto_topup` — recurring top-up configuration.
- `streams_by_employee_paginated`, `streams_in_range`, `stream_count_by_employer`,
  `stream_count_by_employee`, `stream_count_by_token` — index queries.
- `version()`, `is_paused()`, `get_pending_admin()`, `admin_nonce()` — contract state queries.
- Token contract: `allowance(owner, spender)`, `burn_from(spender, from, amount)`,
  `propose_admin`/`accept_admin`, `admin_nonce`, and an upgrade path.
- `.github/labels.yml` plus `scripts/sync-labels.sh` — single source of truth for repository labels.
- `scripts/export-streams.sh` — CSV/JSON export of an employer's streams for payroll reconciliation.
- `docs/migration/v0.1-to-v0.2.md` — migration guide for integrators.
- Responsible disclosure rewards programme, Hall of Fame, and response SLA in `SECURITY.md`.

### Changed

- **Breaking:** `create_stream` takes a new required `low_balance_threshold` argument.
- **Breaking:** `pause_contract` and `unpause_contract` take a required `nonce` argument.
- **Breaking:** `upgrade` takes a required `nonce` argument.
- **Breaking:** token `burn` no longer takes an `admin` argument — burning is authorised by the
  holder or by a spender with an allowance (`burn_from`).
- **Breaking:** token `mint` takes a required `nonce` argument.
- **Breaking:** token `approve` takes a required `expiration_ledger` argument — allowances expire.
- **Breaking:** `initialize` reverts on a second call ("already initialized", audit LOW-02).
- **Breaking:** the `Stream` struct gained `pending_accrual` and `low_balance_threshold` fields.
- **Breaking:** panic messages were replaced by stable `E###` error codes (E007–E029).
- **Breaking:** event topics are prefixed with the schema version `v1`; `stream_cancelled` now
  reports the employee payout and the employer refund.
- **Breaking:** `create_streams_batch` is capped at 50 streams (E029), and its failure messages
  are prefixed with the offending stream index.
- `withdraw` transitions a stream to `Exhausted` once `stop_time` has passed and nothing is claimable.
- `top_up` reverts with `E004` instead of overflowing when a deposit exceeds `i128` range.

### Migration

Upgrading from v0.1.0? Every breaking change, new error code, and event schema change is listed
with before/after examples in the [v0.1 → v0.2 migration guide](docs/migration/v0.1-to-v0.2.md).

## [0.1.0] - 2026-04-24

### Added

- Initial `StreamContract` with `create_stream`, `withdraw`, `top_up`, `pause_stream`, `resume_stream`, and `cancel_stream` entrypoints
- Initial `TokenContract` (SEP-41 compliant) with `initialize`, `transfer`, `transfer_from`, `approve`, `mint`, and `burn`
- Per-second salary streaming with configurable `rate_per_second` and optional `stop_time`
- `claimable` and `claimable_at` query functions for real-time and projected earnings
- `create_streams_batch` for atomic multi-stream creation in a single transaction
- Multi-token support — each stream accepts any SEP-41 compliant token address
- Per-employer and per-employee stream index (`streams_by_employer`, `streams_by_employee`)
- Emergency contract-level `pause_contract` / `unpause_contract` for admin
- Admin-governed minimum deposit enforcement (`set_min_deposit`)
- Contract upgrade path via `upgrade` (WASM hash swap) and `migrate` (no-op base hook)
- Reentrancy guard (`locked` flag) on `withdraw` as defence-in-depth (issue #1)
- Checked arithmetic throughout `claimable_amount` to prevent silent overflow (issue #2)
- Zero-rate validation — `create_stream` rejects `rate_per_second = 0` with `E001` (issue #3)
- Graceful `withdraw` on exhausted streams returns `0` instead of panicking (issue #10)
- `top_up` rejects cancelled (`E005`) and exhausted (`E006`) streams (issue #11)
- Pause/resume time-accounting — paused intervals excluded from claimable calculation (issue #15)
- `StreamStatus` lifecycle: `Active → Paused → Active`, `Active → Cancelled`, `Active → Exhausted`
- On-chain events: `created`, `withdraw`, `status`, `topup`, `paused`
- Rust toolchain pinned via `rust-toolchain.toml`
- Docker-based local development environment (`docker-compose.yml`)
- GitHub Actions CI with build, test, lint, and `cargo audit` vulnerability scanning
- Branch protection script (`scripts/protect-main.sh`)
- Security audit report (Trail of Bits, 2026-04-23) — all high/medium findings resolved
- Gas optimisation pass: `withdraw` CPU instructions reduced ~19%, `claimable` ~21%
- Upgrade guide (`docs/upgrade-guide.md`)

### Security

- `top_up` now verifies `employer == stream.employer` before token transfer (audit HIGH-01)
- `claimable_amount` uses `checked_mul` to prevent overflow; panics with `E004` on overflow (audit MED-01)
- Reentrancy analysis documented; `locked` guard added as defence-in-depth (audit MED-02)

[Unreleased]: https://github.com/veracindarella/paystream-contracts/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/veracindarella/paystream-contracts/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/veracindarella/paystream-contracts/releases/tag/v0.1.0
