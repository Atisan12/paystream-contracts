// SPDX-License-Identifier: Apache-2.0

use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
pub enum TokenDataKey {
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
    Admin,
    PendingAdmin,
    AdminNonce,
    /// TOK-15: admin-configurable supply cap; 0 (or unset) means uncapped.
    MaxSupply,
    /// TOK-18: contract version stamped on `initialize` and `migrate`.
    Version,
    /// TOK-18: upgrade scheduled by `upgrade`, awaiting its time-lock.
    PendingUpgrade,
}

/// TOK-18: a WASM upgrade waiting for its time-lock to elapse.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: BytesN<32>,
    pub unlock_time: u64,
}
