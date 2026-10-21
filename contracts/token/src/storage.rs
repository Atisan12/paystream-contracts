// SPDX-License-Identifier: Apache-2.0

use crate::types::{TokenDataKey, TokenMetadata};
use soroban_sdk::{Address, Env, String};

/// Defaults returned by `decimals` / `name` / `symbol` until the admin calls
/// `set_metadata`. 7 decimals matches Stellar classic assets and the SAC.
pub const DEFAULT_DECIMALS: u32 = 7;
pub const DEFAULT_NAME: &str = "PayStream Token";
pub const DEFAULT_SYMBOL: &str = "PST";

pub fn balance_of(env: &Env, owner: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&TokenDataKey::Balance(owner.clone()))
        .unwrap_or(0)
}

pub fn set_balance(env: &Env, owner: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&TokenDataKey::Balance(owner.clone()), &amount);
}

/// Returns `(amount, expiration_ledger)`; `(0, 0)` if no allowance is stored.
/// The amount is returned even if expired; callers decide how to treat expiry.
pub fn allowance(env: &Env, owner: &Address, spender: &Address) -> (i128, u32) {
    env.storage()
        .temporary()
        .get(&TokenDataKey::Allowance(owner.clone(), spender.clone()))
        .unwrap_or((0, 0))
}

pub fn set_allowance(
    env: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
    expiration_ledger: u32,
) {
    let key = TokenDataKey::Allowance(owner.clone(), spender.clone());
    env.storage()
        .temporary()
        .set(&key, &(amount, expiration_ledger));
    // SEP-41: keep the allowance entry alive until its expiration ledger.
    if amount > 0 {
        let live_for = expiration_ledger.saturating_sub(env.ledger().sequence());
        if live_for > 0 {
            env.storage().temporary().extend_ttl(&key, live_for, live_for);
        }
    }
}

pub fn metadata(env: &Env) -> TokenMetadata {
    env.storage()
        .instance()
        .get(&TokenDataKey::Metadata)
        .unwrap_or_else(|| TokenMetadata {
            decimal: DEFAULT_DECIMALS,
            name: String::from_str(env, DEFAULT_NAME),
            symbol: String::from_str(env, DEFAULT_SYMBOL),
        })
}

pub fn has_metadata(env: &Env) -> bool {
    env.storage().instance().has(&TokenDataKey::Metadata)
}

pub fn set_metadata(env: &Env, metadata: &TokenMetadata) {
    env.storage().instance().set(&TokenDataKey::Metadata, metadata);
}

pub fn total_supply(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&TokenDataKey::TotalSupply)
        .unwrap_or(0)
}

pub fn set_total_supply(env: &Env, supply: i128) {
    env.storage()
        .instance()
        .set(&TokenDataKey::TotalSupply, &supply);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&TokenDataKey::Admin)
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&TokenDataKey::Admin)
        .expect("admin not set")
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&TokenDataKey::Admin, admin);
}

pub fn get_pending_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&TokenDataKey::PendingAdmin)
}

pub fn set_pending_admin(env: &Env, admin: &Address) {
    env.storage()
        .instance()
        .set(&TokenDataKey::PendingAdmin, admin);
}

pub fn clear_pending_admin(env: &Env) {
    env.storage().instance().remove(&TokenDataKey::PendingAdmin);
}

pub fn admin_nonce(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&TokenDataKey::AdminNonce)
        .unwrap_or(0)
}

/// Asserts `nonce` matches the stored admin nonce, then increments it.
pub fn consume_admin_nonce(env: &Env, nonce: u64) {
    let current = admin_nonce(env);
    assert!(nonce == current, "invalid nonce");
    env.storage()
        .instance()
        .set(&TokenDataKey::AdminNonce, &(current + 1));
}
