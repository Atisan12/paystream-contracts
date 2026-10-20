# Auth Audit — `paystream-stream` Contract

**Date:** 2026-10-18  
**Auditor:** Internal (SEC-25 / issue #263)  
**Scope:** Every public function in `contracts/stream/src/lib.rs`  
**Method:** Manual review of each entry point for correct `require_auth()` placement  
**Status:** ✅ No gaps found

---

## Summary

All state-mutating public functions call `require_auth()` on the correct party before
performing any state read or write. Read-only functions that expose no privileged data
require no auth — this is intentional and consistent with Soroban best practice.

---

## Function-by-Function Review

### Admin functions

| Function | Required Signer | Auth Call | Verification |
|---|---|---|---|
| `initialize(admin)` | `admin` | `admin.require_auth()` | ✅ Called before `set_admin`; panics if admin is already set so replaying is impossible |
| `propose_admin(new_admin, nonce)` | current admin (via `get_admin`) | `current.require_auth()` | ✅ Current admin fetched from storage, then `.require_auth()` before any mutation |
| `accept_admin(new_admin, nonce)` | `new_admin` (incoming admin) | `new_admin.require_auth()` | ✅ Called on the nominee before validating pending admin state |
| `pause_contract(nonce)` | current admin (via `get_admin`) | `admin.require_auth()` | ✅ Admin fetched, authed, nonce consumed, then paused |
| `unpause_contract(nonce)` | current admin (via `get_admin`) | `admin.require_auth()` | ✅ Same pattern as `pause_contract` |
| `set_min_deposit(admin, nonce, amount)` | `admin` | `admin.require_auth()` | ✅ Auth + equality check against stored admin |
| `expire_streams(admin, stream_ids)` | `admin` | `admin.require_auth()` | ✅ Auth + equality check against stored admin before iterating |
| `upgrade(new_wasm_hash, nonce)` | current admin (via instance storage) | `admin.require_auth()` | ✅ Admin loaded from storage, authed, nonce consumed before WASM swap |
| `propose_upgrade(new_wasm_hash, nonce)` | current admin | `admin.require_auth()` | ✅ Same as `upgrade` |
| `execute_upgrade(nonce)` | current admin | `admin.require_auth()` | ✅ Auth before checking timelock and executing |
| `cancel_upgrade(nonce)` | current admin | `admin.require_auth()` | ✅ Auth before clearing pending upgrade |
| `migrate(admin)` | `admin` | `admin.require_auth()` | ✅ Auth + equality check against stored admin |
| `set_max_creates_per_ledger(admin, nonce, max)` | `admin` | `admin.require_auth()` | ✅ Auth + equality check against stored admin; nonce consumed |

### Employer functions

| Function | Required Signer | Auth Call | Verification |
|---|---|---|---|
| `create_stream(employer, ...)` | `employer` | `employer.require_auth()` | ✅ First line of function body; checked before any token transfer |
| `create_streams_batch(employer, params)` | `employer` | `employer.require_auth()` | ✅ First line; batch validation runs after auth |
| `create_stream_from_template(employer, employee, template_id)` | `employer` (delegated to `create_stream`) | Called inside `create_stream` | ✅ Template load is read-only; auth is enforced by the delegated `create_stream` call |
| `top_up(employer, stream_id, amount, ...)` | `employer` | `employer.require_auth()` | ✅ First call; employer equality against stored stream checked after |
| `set_low_balance_threshold(employer, stream_id, threshold)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `set_auto_topup(employer, stream_id, ...)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `cancel_auto_topup(employer, stream_id)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `pause_stream(employer, stream_id)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `pause_stream_batch(employer, stream_ids)` | `employer` | `employer.require_auth()` | ✅ Auth before batch loop |
| `resume_stream(employer, stream_id)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `resume_stream_batch(employer, stream_ids)` | `employer` | `employer.require_auth()` | ✅ Auth before batch loop |
| `update_rate(employer, stream_id, new_rate)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `cancel_stream(employer, stream_id)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality check |
| `cancel_streams_batch(employer, stream_ids)` | `employer` | `employer.require_auth()` | ✅ Auth before batch loop |
| `transfer_stream(employer, stream_id, new_employer)` | `employer` | `employer.require_auth()` | ✅ Auth then employer equality + new_employer ≠ employee check |
| `save_template(employer, template_id, params)` | `employer` | `employer.require_auth()` | ✅ Auth before template write |

### Employee functions

| Function | Required Signer | Auth Call | Verification |
|---|---|---|---|
| `withdraw(employee, stream_id)` | `employee` | `employee.require_auth()` | ✅ Auth before stream load; receipt owner also verified against `employee` |
| `withdraw_all(employee)` | `employee` | `employee.require_auth()` | ✅ Auth once before iterating all employee streams; receipt owner re-checked per stream |
| `transfer_receipt(from, to, stream_id)` | `from` (current receipt holder) | `from.require_auth()` | ✅ Auth then receipt owner equality check before transferring |

### Public / permissionless functions (no auth required)

These functions are intentionally callable by anyone; they either return read-only data
or perform permissionless settlement that benefits the stream's parties.

| Function | Rationale |
|---|---|
| `get_stream(stream_id)` | Read-only; no state mutation |
| `get_streams_batch(stream_ids)` | Read-only |
| `stream_status(stream_id)` | Read-only |
| `claimable(stream_id)` | Read-only |
| `claimable_batch(stream_ids)` | Read-only |
| `claimable_at(stream_id, timestamp)` | Read-only projection |
| `receipt_owner(stream_id)` | Read-only |
| `get_auto_topup_config(stream_id)` | Read-only |
| `get_auto_topup(stream_id)` | Read-only alias |
| `get_pending_admin()` | Read-only |
| `admin_nonce()` | Read-only |
| `streams_by_employer(employer)` | Read-only index query |
| `streams_by_employee(employee)` | Read-only index query |
| `streams_by_employee_paginated(employee, offset, limit)` | Read-only paginated query |
| `streams_in_range(from_ts, to_ts)` | Read-only time-range query |
| `stream_count()` | Read-only counter |
| `stream_count_by_employer(employer)` | Read-only counter |
| `stream_count_by_employee(employee)` | Read-only counter |
| `stream_count_by_token(token)` | Read-only counter |
| `is_paused()` | Read-only |
| `admin()` | Read-only; admin address is not secret |
| `version()` | Read-only |
| `list_templates(employer)` | Read-only |
| `settle_stream(stream_id)` | Permissionless keeper; settles an expired stream in favour of its parties; no party can extract funds they are not owed |

---

## Auth Placement Pattern

The codebase follows a consistent pattern:

1. Call `require_auth()` as the **first statement** in every state-mutating function.
2. Load stream state **after** auth to avoid paying load costs for unauthenticated calls.
3. Re-assert caller identity against stored stream fields (`assert_eq!(stream.employer, employer)`)
   as a secondary check — ensures the authed address is also the *correct* party for that specific stream.
4. Consume admin nonces atomically with the auth check to prevent replay attacks.

This two-layer model (auth + field equality) means a valid Stellar keypair that is not
the stream's employer cannot affect that stream, even if it can authenticate.

---

## Gaps Found

**None.** No missing or misplaced `require_auth()` calls were identified during this audit.

---

## CodeQL Coverage

The custom CodeQL query `.github/codeql/queries/MissingRequireAuth.ql` enforces this
at the CI level. It flags any `impl` function that:
- mutates storage (calls `set_*`, `save_*`, `remove_*`, or similar helpers), and
- does not call `require_auth` or a recognized auth-checking helper.

The query fires on each push via `.github/workflows/codeql.yml`. Any regression
(new function missing auth) will block the PR.

A dedicated `StorageMutationWithoutAuth.ql` query was also added (see
`.github/codeql/queries/StorageMutationWithoutAuth.ql`) that focuses specifically
on direct `env.storage().*.set()` and `.remove()` calls without a preceding auth
check in the same function scope.

---

## References

- Issue: #263 (SEC-25)
- Related: `.github/codeql/queries/MissingRequireAuth.ql`
- Related: `.github/codeql/queries/StorageMutationWithoutAuth.ql`
- Soroban auth model: https://developers.stellar.org/docs/build/smart-contracts/example-contracts/auth
