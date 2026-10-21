// SPDX-License-Identifier: Apache-2.0

#![no_std]

mod storage;
mod types;

#[cfg(test)]
mod test;

use crate::storage::{
    admin_nonce, allowance, balance_of, clear_pending_admin, clear_pending_upgrade,
    consume_admin_nonce, extend_instance_ttl, get_admin, get_pending_admin, get_pending_upgrade,
    has_admin, max_supply, set_admin, set_allowance, set_balance, set_max_supply,
    set_pending_admin, set_pending_upgrade, set_total_supply, set_version, total_supply, version,
    UPGRADE_TIMELOCK,
};
use crate::types::PendingUpgrade;
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

pub use crate::storage::{TTL_EXTEND_TO, TTL_THRESHOLD};

/// TOK-18: current token contract version, stamped on `initialize` and `migrate`.
pub const TOKEN_CONTRACT_VERSION: u32 = 1;

/// T001: token arithmetic overflow.
const ERR_OVERFLOW: &str = "T001: token arithmetic overflow";
/// T002: mint would exceed the configured max supply.
const ERR_MAX_SUPPLY: &str = "T002: max supply exceeded";
/// T003: no upgrade has been scheduled.
const ERR_NO_PENDING_UPGRADE: &str = "T003: no pending upgrade";
/// T004: upgrade time-lock has not yet elapsed.
const ERR_UPGRADE_TIMELOCK_ACTIVE: &str = "T004: upgrade timelock active";

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// Initialise the token contract with an admin and an initial supply.
    ///
    /// Mints `initial_supply` tokens directly to `admin`. Must be called once
    /// after deployment before any other function.
    ///
    /// # Parameters
    /// - `admin` — address that becomes the token admin (can mint)
    /// - `initial_supply` — tokens minted to `admin` on initialisation
    ///
    /// # Errors
    /// - Panics if the contract has already been initialised ("already initialized")
    /// - Panics if `admin` auth fails
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        admin.require_auth();
        assert!(!has_admin(&env), "already initialized");
        set_admin(&env, &admin);
        set_balance(&env, &admin, initial_supply);
        set_total_supply(&env, initial_supply);
        set_version(&env, TOKEN_CONTRACT_VERSION);
        extend_instance_ttl(&env);
    }

    /// Return the total token supply.
    ///
    /// # Returns
    /// Current total supply as `i128`.
    pub fn total_supply(env: Env) -> i128 {
        total_supply(&env)
    }

    /// Return the token balance of `owner`.
    ///
    /// # Parameters
    /// - `owner` — address to query
    ///
    /// # Returns
    /// Balance as `i128`; 0 if the address has never held tokens.
    pub fn balance(env: Env, owner: Address) -> i128 {
        balance_of(&env, &owner)
    }

    /// Transfer `amount` tokens from `from` to `to`.
    ///
    /// # Parameters
    /// - `from` — sender (requires auth)
    /// - `to` — recipient
    /// - `amount` — number of tokens to transfer (must be > 0)
    ///
    /// # Errors
    /// - Panics if `amount` ≤ 0
    /// - Panics if `from` has insufficient balance
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        extend_instance_ttl(&env);
        assert!(amount > 0, "amount must be positive");
        let from_bal = balance_of(&env, &from);
        assert!(from_bal >= amount, "insufficient balance");
        set_balance(&env, &from, from_bal - amount);
        let to_bal = balance_of(&env, &to)
            .checked_add(amount)
            .expect(ERR_OVERFLOW);
        set_balance(&env, &to, to_bal);
    }

    /// Approve `spender` to transfer up to `amount` tokens on behalf of `owner`.
    ///
    /// Overwrites any existing allowance. Set `amount` to 0 to revoke.
    ///
    /// # Parameters
    /// - `owner` — token owner (requires auth)
    /// - `spender` — address being approved
    /// - `amount` — new allowance
    /// - `expiration_ledger` — last ledger sequence at which the allowance is valid
    ///
    /// # Errors
    /// - Panics if `amount` > 0 and `expiration_ledger` is before the current ledger
    pub fn approve(
        env: Env,
        owner: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) {
        owner.require_auth();
        extend_instance_ttl(&env);
        assert!(
            amount == 0 || expiration_ledger >= env.ledger().sequence(),
            "expiration_ledger is in the past"
        );
        set_allowance(&env, &owner, &spender, amount, expiration_ledger);
    }

    /// Return the allowance granted by `owner` to `spender` (SEP-41).
    ///
    /// # Returns
    /// Remaining allowance; 0 if none exists or it has expired.
    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        allowance(&env, &owner, &spender).0
    }

    /// Transfer `amount` tokens from `from` to `to` using `spender`'s allowance.
    ///
    /// # Parameters
    /// - `spender` — address with an existing allowance (requires auth)
    /// - `from` — token owner
    /// - `to` — recipient
    /// - `amount` — number of tokens to transfer
    ///
    /// # Errors
    /// - Panics if `spender`'s allowance for `from` has expired
    /// - Panics if `spender`'s allowance for `from` is insufficient
    /// - Panics if `from` has insufficient balance
    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        extend_instance_ttl(&env);
        let (allowed, expiration_ledger) = allowance(&env, &from, &spender);
        assert!(
            env.ledger().sequence() <= expiration_ledger,
            "allowance expired"
        );
        assert!(allowed >= amount, "allowance exceeded");
        let from_bal = balance_of(&env, &from);
        assert!(from_bal >= amount, "insufficient balance");
        set_allowance(&env, &from, &spender, allowed - amount, expiration_ledger);
        set_balance(&env, &from, from_bal - amount);
        let to_bal = balance_of(&env, &to)
            .checked_add(amount)
            .expect(ERR_OVERFLOW);
        set_balance(&env, &to, to_bal);
    }

    /// Mint `amount` new tokens to `to`, increasing total supply.
    ///
    /// Only the admin may call this function.
    ///
    /// # Parameters
    /// - `admin` — must match the stored admin (requires auth)
    /// - `to` — recipient of minted tokens
    /// - `amount` — number of tokens to mint (must be > 0)
    /// - `nonce` — current admin nonce; consumed for replay protection
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    /// - Panics if `nonce` does not match the stored admin nonce
    /// - Panics if `amount` ≤ 0
    /// - T001 if the balance or total supply would overflow
    /// - T002 if a max supply is set and `total_supply + amount` exceeds it
    pub fn mint(env: Env, admin: Address, to: Address, amount: i128, nonce: u64) {
        admin.require_auth();
        assert_eq!(get_admin(&env), admin, "not admin");
        consume_admin_nonce(&env, nonce);
        assert!(amount > 0, "amount must be positive");
        let new_supply = total_supply(&env).checked_add(amount).expect(ERR_OVERFLOW);
        let cap = max_supply(&env);
        assert!(cap == 0 || new_supply <= cap, "{}", ERR_MAX_SUPPLY);
        let to_bal = balance_of(&env, &to)
            .checked_add(amount)
            .expect(ERR_OVERFLOW);
        set_balance(&env, &to, to_bal);
        set_total_supply(&env, new_supply);
        extend_instance_ttl(&env);
    }

    /// Set the maximum total supply enforced by [`mint`] (TOK-15).
    ///
    /// # Parameters
    /// - `admin` — must match the stored admin (requires auth)
    /// - `nonce` — current admin nonce; consumed for replay protection
    /// - `max` — supply cap; 0 means uncapped
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    /// - Panics if `nonce` does not match the stored admin nonce
    /// - Panics if `max` is negative
    pub fn set_max_supply(env: Env, admin: Address, nonce: u64, max: i128) {
        admin.require_auth();
        assert_eq!(get_admin(&env), admin, "not admin");
        consume_admin_nonce(&env, nonce);
        assert!(max >= 0, "max supply must be non-negative");
        set_max_supply(&env, max);
        extend_instance_ttl(&env);
    }

    /// Return the configured max supply; 0 means uncapped.
    pub fn get_max_supply(env: Env) -> i128 {
        max_supply(&env)
    }

    /// Return the current admin nonce expected by [`mint`].
    pub fn admin_nonce(env: Env) -> u64 {
        admin_nonce(&env)
    }

    /// Schedule a WASM upgrade behind a 24-hour time-lock (TOK-18).
    ///
    /// Balances, allowances and total supply live in contract storage and are
    /// preserved across the upgrade. After [`UPGRADE_TIMELOCK`] seconds call
    /// [`execute_upgrade`], then [`migrate`].
    ///
    /// # Parameters
    /// - `admin` — must match the stored admin (requires auth)
    /// - `nonce` — current admin nonce (replay protection)
    /// - `new_wasm_hash` — 32-byte hash of the uploaded WASM blob
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    /// - Panics if `nonce` does not match the stored admin nonce
    pub fn upgrade(env: Env, admin: Address, nonce: u64, new_wasm_hash: BytesN<32>) {
        admin.require_auth();
        assert_eq!(get_admin(&env), admin, "not admin");
        consume_admin_nonce(&env, nonce);
        let unlock_time = env.ledger().timestamp() + UPGRADE_TIMELOCK;
        set_pending_upgrade(
            &env,
            &PendingUpgrade {
                wasm_hash: new_wasm_hash,
                unlock_time,
            },
        );
        extend_instance_ttl(&env);
    }

    /// Apply the upgrade scheduled by [`upgrade`] once its time-lock has elapsed.
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    /// - Panics if `nonce` does not match the stored admin nonce
    /// - T003 if no upgrade is pending
    /// - T004 if the time-lock has not yet elapsed
    pub fn execute_upgrade(env: Env, admin: Address, nonce: u64) {
        admin.require_auth();
        assert_eq!(get_admin(&env), admin, "not admin");
        consume_admin_nonce(&env, nonce);
        let pending = get_pending_upgrade(&env).expect(ERR_NO_PENDING_UPGRADE);
        assert!(
            env.ledger().timestamp() >= pending.unlock_time,
            "{}",
            ERR_UPGRADE_TIMELOCK_ACTIVE
        );
        clear_pending_upgrade(&env);
        env.deployer()
            .update_current_contract_wasm(pending.wasm_hash);
    }

    /// Cancel a scheduled upgrade before it is executed.
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    /// - Panics if `nonce` does not match the stored admin nonce
    /// - T003 if no upgrade is pending
    pub fn cancel_upgrade(env: Env, admin: Address, nonce: u64) {
        admin.require_auth();
        assert_eq!(get_admin(&env), admin, "not admin");
        consume_admin_nonce(&env, nonce);
        get_pending_upgrade(&env).expect(ERR_NO_PENDING_UPGRADE);
        clear_pending_upgrade(&env);
    }

    /// Return the upgrade scheduled by [`upgrade`], if any.
    pub fn pending_upgrade(env: Env) -> Option<PendingUpgrade> {
        get_pending_upgrade(&env)
    }

    /// Post-upgrade migration hook: stamps [`TOKEN_CONTRACT_VERSION`].
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match the stored admin
    /// - Panics if `nonce` does not match the stored admin nonce
    pub fn migrate(env: Env, admin: Address, nonce: u64) {
        admin.require_auth();
        assert_eq!(get_admin(&env), admin, "not admin");
        consume_admin_nonce(&env, nonce);
        set_version(&env, TOKEN_CONTRACT_VERSION);
        extend_instance_ttl(&env);
    }

    /// Return the stored contract version; 0 if never stamped.
    pub fn version(env: Env) -> u32 {
        version(&env)
    }

    /// Step 1 of two-step admin transfer: current admin nominates `new_admin`.
    ///
    /// # Parameters
    /// - `new_admin` — address being nominated as the next admin
    ///
    /// # Errors
    /// - Panics if the current admin auth fails
    pub fn propose_admin(env: Env, new_admin: Address) {
        get_admin(&env).require_auth();
        set_pending_admin(&env, &new_admin);
    }

    /// Step 2 of two-step admin transfer: nominated address accepts and becomes admin.
    ///
    /// # Parameters
    /// - `new_admin` — must match the address set by [`propose_admin`] (requires auth)
    ///
    /// # Errors
    /// - Panics if there is no pending admin
    /// - Panics if `new_admin` does not match the pending admin
    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();
        let pending = get_pending_admin(&env).expect("no pending admin");
        assert_eq!(pending, new_admin, "not pending admin");
        set_admin(&env, &new_admin);
        clear_pending_admin(&env);
    }

    /// Burn `amount` tokens from `from`'s own balance, reducing total supply.
    ///
    /// # Parameters
    /// - `from` — address whose tokens are burned (requires auth)
    /// - `amount` — number of tokens to burn (must be > 0)
    ///
    /// # Errors
    /// - Panics if `amount` ≤ 0
    /// - Panics if `from` has insufficient balance
    pub fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        extend_instance_ttl(&env);
        assert!(amount > 0, "amount must be positive");
        let bal = balance_of(&env, &from);
        assert!(bal >= amount, "insufficient balance");
        set_balance(&env, &from, bal - amount);
        set_total_supply(&env, total_supply(&env) - amount);
    }

    /// Burn `amount` tokens from `from` using `spender`'s allowance.
    ///
    /// Reduces both `from`'s balance and the total supply.
    ///
    /// # Parameters
    /// - `spender` — address with an existing allowance (requires auth)
    /// - `from` — token owner whose tokens are burned
    /// - `amount` — number of tokens to burn (must be > 0)
    ///
    /// # Errors
    /// - Panics if `amount` ≤ 0
    /// - Panics if `spender`'s allowance for `from` has expired
    /// - Panics if `spender`'s allowance for `from` is insufficient
    /// - Panics if `from` has insufficient balance
    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        extend_instance_ttl(&env);
        assert!(amount > 0, "amount must be positive");
        let (allowed, expiration_ledger) = allowance(&env, &from, &spender);
        assert!(
            env.ledger().sequence() <= expiration_ledger,
            "allowance expired"
        );
        assert!(allowed >= amount, "allowance exceeded");
        let bal = balance_of(&env, &from);
        assert!(bal >= amount, "insufficient balance");
        set_allowance(&env, &from, &spender, allowed - amount, expiration_ledger);
        set_balance(&env, &from, bal - amount);
        set_total_supply(&env, total_supply(&env) - amount);
    }
}
