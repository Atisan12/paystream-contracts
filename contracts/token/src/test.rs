// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::{TokenContract, TokenContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Env, Vec};

fn setup() -> (Env, TokenContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(TokenContract, ());
    let client = TokenContractClient::new(&env, &id);
    (env, client)
}

#[test]
fn test_initialize() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000);
    assert_eq!(client.total_supply(), 1_000_000);
    assert_eq!(client.balance(&admin), 1_000_000);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_token_initialize_cannot_be_called_twice() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000);
    // Second call must panic with "already initialized"
    client.initialize(&admin, &500_000);
}

#[test]
fn test_transfer() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.transfer(&admin, &user, &400);
    assert_eq!(client.balance(&admin), 600);
    assert_eq!(client.balance(&user), 400);
}

#[test]
fn test_mint_and_burn() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.mint(&admin, &user, &500, &0);
    assert_eq!(client.total_supply(), 1_500);
    // holder burns their own tokens
    client.burn(&user, &200);
    assert_eq!(client.total_supply(), 1_300);
    assert_eq!(client.balance(&user), 300);
}

#[test]
fn test_burn_from_with_allowance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.transfer(&admin, &user, &500);
    client.approve(&user, &spender, &300, &100);
    client.burn_from(&spender, &user, &200);
    assert_eq!(client.balance(&user), 300);
    assert_eq!(client.total_supply(), 800);
}

#[test]
#[should_panic(expected = "allowance exceeded")]
fn test_burn_from_exceeds_allowance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.transfer(&admin, &user, &500);
    client.approve(&user, &spender, &100, &100);
    client.burn_from(&spender, &user, &200);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn test_burn_insufficient_balance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &100);
    client.transfer(&admin, &user, &50);
    client.burn(&user, &200);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn test_transfer_overdraft() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &100);
    client.transfer(&admin, &user, &999);
}

#[test]
fn test_admin_transfer_flow() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.propose_admin(&new_admin, &0);
    client.accept_admin(&new_admin);
    client.mint(&new_admin, &user, &10, &1);
    assert_eq!(client.balance(&user), 10);
}

#[test]
#[should_panic(expected = "not pending admin")]
fn test_accept_admin_wrong_address() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.propose_admin(&Address::generate(&env), &0);
    client.accept_admin(&Address::generate(&env));
}

#[test]
#[should_panic(expected = "no pending admin")]
fn test_accept_admin_without_pending() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.accept_admin(&Address::generate(&env));
}

#[test]
fn test_mint_increments_nonce() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    assert_eq!(client.admin_nonce(), 0);
    client.mint(&admin, &user, &10, &0);
    client.mint(&admin, &user, &10, &1);
    assert_eq!(client.admin_nonce(), 2);
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_mint_replay_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.mint(&admin, &user, &10, &0);
    client.mint(&admin, &user, &10, &0);
}

#[test]
#[should_panic(expected = "T001: token arithmetic overflow")]
fn test_mint_overflow_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &i128::MAX);
    client.mint(&admin, &admin, &1, &0);
}

#[test]
fn test_allowance_not_expired_works() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    let to = Address::generate(&env);
    client.initialize(&admin, &1_000);
    let exp = env.ledger().sequence() + 5;
    client.approve(&admin, &spender, &300, &exp);
    assert_eq!(client.allowance(&admin, &spender), (300, exp));
    env.ledger().with_mut(|l| l.sequence_number = exp);
    client.transfer_from(&spender, &admin, &to, &100);
    assert_eq!(client.balance(&to), 100);
    assert_eq!(client.allowance(&admin, &spender), (200, exp));
}

#[test]
#[should_panic(expected = "allowance expired")]
fn test_transfer_from_expired_allowance_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    let to = Address::generate(&env);
    client.initialize(&admin, &1_000);
    let exp = env.ledger().sequence() + 5;
    client.approve(&admin, &spender, &300, &exp);
    env.ledger().with_mut(|l| l.sequence_number = exp + 1);
    client.transfer_from(&spender, &admin, &to, &100);
}

#[test]
#[should_panic(expected = "allowance expired")]
fn test_burn_from_expired_allowance_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    let exp = env.ledger().sequence() + 5;
    client.approve(&admin, &spender, &300, &exp);
    env.ledger().with_mut(|l| l.sequence_number = exp + 1);
    client.burn_from(&spender, &admin, &100);
}

// ---------------------------------------------------------------------------
// TOK-10 – upgrade mechanism
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn test_upgrade_non_admin_rejected() {
    let env = Env::default();
    let id = env.register(TokenContract, ());
    let client = TokenContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin, &1_000);
    // Drop mocked auths: upgrade must require the stored admin's signature.
    env.set_auths(&[]);
    client.upgrade(&soroban_sdk::BytesN::from_array(&env, &[0u8; 32]), &0);
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_upgrade_wrong_nonce_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.upgrade(&soroban_sdk::BytesN::from_array(&env, &[0u8; 32]), &5);
}

#[test]
#[should_panic(expected = "not admin")]
fn test_migrate_non_admin_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.migrate(&Address::generate(&env));
}

// Requires a release WASM build of the token contract:
//   cargo build -p paystream-token --target wasm32v1-none --release
//   cargo test -p paystream-token --features wasm-tests
#[cfg(feature = "wasm-tests")]
mod token_wasm {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/paystream_token.wasm");
}

#[cfg(feature = "wasm-tests")]
#[test]
fn test_upgrade_preserves_balances_and_supply() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.transfer(&admin, &user, &250);

    let new_wasm_hash = env.deployer().upload_contract_wasm(token_wasm::WASM);
    client.upgrade(&new_wasm_hash, &0);
    client.migrate(&admin);

    assert_eq!(client.total_supply(), 1_000);
    assert_eq!(client.balance(&admin), 750);
    assert_eq!(client.balance(&user), 250);
    assert_eq!(client.admin_nonce(), 1);
}

// --- TOK-11: burn_from ---

#[test]
#[should_panic(expected = "insufficient balance")]
fn test_burn_from_insufficient_balance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.transfer(&admin, &user, &100);
    client.approve(&user, &spender, &500, &100);
    client.burn_from(&spender, &user, &200);
}

#[test]
fn test_burn_from_reduces_allowance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.transfer(&admin, &user, &500);
    client.approve(&user, &spender, &300, &100);
    client.burn_from(&spender, &user, &200);
    assert_eq!(client.allowance(&user, &spender), (100, 100));
}

// --- TOK-12: admin nonce ---

#[test]
fn test_propose_admin_consumes_nonce() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    assert_eq!(client.get_admin_nonce(), 0);
    client.propose_admin(&Address::generate(&env), &0);
    assert_eq!(client.get_admin_nonce(), 1);
}

#[test]
#[should_panic(expected = "invalid nonce")]
fn test_propose_admin_stale_nonce_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.propose_admin(&Address::generate(&env), &0);
    client.propose_admin(&Address::generate(&env), &0);
}

// --- TOK-13: pause / unpause ---

#[test]
#[should_panic(expected = "T002: token is paused")]
fn test_transfer_while_paused_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.pause_token(&admin, &0);
    client.transfer(&admin, &Address::generate(&env), &10);
}

#[test]
#[should_panic(expected = "T002: token is paused")]
fn test_transfer_from_while_paused_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.approve(&admin, &spender, &100, &100);
    client.pause_token(&admin, &0);
    client.transfer_from(&spender, &admin, &Address::generate(&env), &10);
}

#[test]
#[should_panic(expected = "T002: token is paused")]
fn test_burn_while_paused_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.pause_token(&admin, &0);
    client.burn(&admin, &10);
}

#[test]
#[should_panic(expected = "T002: token is paused")]
fn test_burn_from_while_paused_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.approve(&admin, &spender, &100, &100);
    client.pause_token(&admin, &0);
    client.burn_from(&spender, &admin, &10);
}

#[test]
fn test_unpause_restores_transfers() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.pause_token(&admin, &0);
    assert!(client.is_paused());
    client.unpause_token(&admin, &1);
    assert!(!client.is_paused());
    assert_eq!(client.get_admin_nonce(), 2);
    client.transfer(&admin, &user, &10);
    assert_eq!(client.balance(&user), 10);
}

#[test]
#[should_panic(expected = "not admin")]
fn test_pause_non_admin_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.pause_token(&Address::generate(&env), &0);
}

// --- TOK-14: mint_to_batch ---

#[test]
fn test_mint_to_batch_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.mint_to_batch(&admin, &0, &vec![&env, (a.clone(), 100), (b.clone(), 250)]);
    assert_eq!(client.balance(&a), 100);
    assert_eq!(client.balance(&b), 250);
    assert_eq!(client.total_supply(), 1_350);
    assert_eq!(client.admin_nonce(), 1);
}

#[test]
#[should_panic(expected = "T001: token arithmetic overflow")]
fn test_mint_to_batch_overflow_reverts() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &(i128::MAX - 10));
    client.mint_to_batch(
        &admin,
        &0,
        &vec![
            &env,
            (Address::generate(&env), 5),
            (Address::generate(&env), 10),
        ],
    );
}

#[test]
#[should_panic(expected = "T003: batch too large")]
fn test_mint_to_batch_cap_exceeded() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    let mut recipients: Vec<(Address, i128)> = Vec::new(&env);
    for _ in 0..51 {
        recipients.push_back((Address::generate(&env), 1));
    }
    client.mint_to_batch(&admin, &0, &recipients);
}
