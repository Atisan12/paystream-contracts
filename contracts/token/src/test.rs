// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

extern crate std;

use crate::{TokenContract, TokenContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env};

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
    client.propose_admin(&new_admin);
    client.accept_admin(&new_admin);
    client.mint(&new_admin, &user, &10, &0);
    assert_eq!(client.balance(&user), 10);
}

#[test]
#[should_panic(expected = "not pending admin")]
fn test_accept_admin_wrong_address() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000);
    client.propose_admin(&Address::generate(&env));
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
    assert_eq!(client.allowance(&admin, &spender), 300);
    assert_eq!(client.allowance_with_expiration(&admin, &spender), (300, exp));
    env.ledger().with_mut(|l| l.sequence_number = exp);
    client.transfer_from(&spender, &admin, &to, &100);
    assert_eq!(client.balance(&to), 100);
    assert_eq!(client.allowance(&admin, &spender), 200);
    assert_eq!(client.allowance_with_expiration(&admin, &spender), (200, exp));
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

// ---------------------------------------------------------------------------
// TOK-20 (#305): SEP-41 compliance
// ---------------------------------------------------------------------------

mod sep41 {
    extern crate std;
    use crate::{TokenContract, TokenContractClient, TransferMuxedData};
    use soroban_sdk::testutils::{Address as _, Events as _, Ledger, MuxedAddress as _};
    use soroban_sdk::xdr::{ContractEventBody, ScVal};
    use soroban_sdk::{symbol_short, Address, Env, IntoVal, MuxedAddress, String, TryFromVal, Val};

    fn setup() -> (Env, TokenContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(TokenContract, ());
        let client = TokenContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &1_000_000);
        (env, client, admin)
    }

    /// Topics and data of the last event emitted by the last invocation.
    fn last_event(env: &Env) -> (std::vec::Vec<Val>, Val) {
        let all = env.events().all();
        let e = all.events().last().expect("an event").clone();
        let ContractEventBody::V0(body) = e.body;
        let topics = body.topics.iter().map(|t: &ScVal| Val::try_from_val(env, t).unwrap()).collect();
        (topics, Val::try_from_val(env, &body.data).unwrap())
    }

    fn assert_topics(env: &Env, got: &[Val], want: &[Val]) {
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(want) {
            assert!(g.shallow_eq(w) || ScVal::try_from_val(env, g).unwrap() == ScVal::try_from_val(env, w).unwrap());
        }
    }

    #[test]
    fn metadata_defaults_then_set_once() {
        let (env, client, admin) = setup();
        assert_eq!(client.decimals(), 7);
        assert_eq!(client.name(), String::from_str(&env, "PayStream Token"));
        assert_eq!(client.symbol(), String::from_str(&env, "PST"));

        client.set_metadata(&admin, &6, &String::from_str(&env, "Pay USD"), &String::from_str(&env, "PUSD"));
        assert_eq!(client.decimals(), 6);
        assert_eq!(client.symbol(), String::from_str(&env, "PUSD"));
        assert!(client
            .try_set_metadata(&admin, &2, &String::from_str(&env, "x"), &String::from_str(&env, "x"))
            .is_err());
    }

    #[test]
    fn set_metadata_rejects_non_admin() {
        let (env, client, _) = setup();
        let stranger = Address::generate(&env);
        assert!(client
            .try_set_metadata(&stranger, &2, &String::from_str(&env, "x"), &String::from_str(&env, "x"))
            .is_err());
    }

    #[test]
    fn allowance_returns_zero_after_expiry() {
        let (env, client, admin) = setup();
        let spender = Address::generate(&env);
        let exp = env.ledger().sequence() + 10;
        client.approve(&admin, &spender, &500, &exp);
        assert_eq!(client.allowance(&admin, &spender), 500);
        env.ledger().with_mut(|l| l.sequence_number = exp + 1);
        assert_eq!(client.allowance(&admin, &spender), 0);
        assert_eq!(client.allowance_with_expiration(&admin, &spender), (500, exp));
    }

    #[test]
    fn approve_rejects_negative_amount() {
        let (env, client, admin) = setup();
        let spender = Address::generate(&env);
        let exp = env.ledger().sequence() + 10;
        assert!(client.try_approve(&admin, &spender, &-1, &exp).is_err());
    }

    #[test]
    fn transfer_from_rejects_negative_amount() {
        let (env, client, admin) = setup();
        let spender = Address::generate(&env);
        let to = Address::generate(&env);
        let exp = env.ledger().sequence() + 10;
        client.approve(&admin, &spender, &100, &exp);
        assert!(client.try_transfer_from(&spender, &admin, &to, &-50).is_err());
        assert_eq!(client.balance(&admin), 1_000_000);
        assert_eq!(client.allowance(&admin, &spender), 100);
    }

    #[test]
    fn transfer_emits_sep41_event() {
        let (env, client, admin) = setup();
        let to = Address::generate(&env);
        client.transfer(&admin, &to, &25);
        let (topics, data) = last_event(&env);
        assert_topics(&env, &topics, &[symbol_short!("transfer").into_val(&env), admin.into_val(&env), to.into_val(&env)]);
        assert_eq!(i128::try_from_val(&env, &data).unwrap(), 25);
    }

    #[test]
    fn transfer_to_muxed_address_credits_base_account_and_reports_id() {
        let (env, client, admin) = setup();
        // Only account (G…) addresses can be muxed.
        let base = MuxedAddress::generate(&env);
        let muxed = MuxedAddress::new(base.clone(), 42);
        let to = muxed.address();
        client.transfer(&admin, &muxed, &10);
        let (_, data) = last_event(&env);
        assert_eq!(
            TransferMuxedData::try_from_val(&env, &data).unwrap(),
            TransferMuxedData { amount: 10, to_muxed_id: 42 }
        );
        assert_eq!(client.balance(&to), 10);
    }

    #[test]
    fn approve_burn_and_mint_emit_events() {
        let (env, client, admin) = setup();
        let spender = Address::generate(&env);
        let exp = env.ledger().sequence() + 10;

        client.approve(&admin, &spender, &7, &exp);
        let (topics, data) = last_event(&env);
        assert_topics(&env, &topics, &[symbol_short!("approve").into_val(&env), admin.into_val(&env), spender.into_val(&env)]);
        assert_eq!(<(i128, u32)>::try_from_val(&env, &data).unwrap(), (7, exp));

        client.burn(&admin, &3);
        let (topics, data) = last_event(&env);
        assert_topics(&env, &topics, &[symbol_short!("burn").into_val(&env), admin.into_val(&env)]);
        assert_eq!(i128::try_from_val(&env, &data).unwrap(), 3);

        client.mint(&admin, &spender, &9, &0);
        let (topics, data) = last_event(&env);
        assert_topics(&env, &topics, &[symbol_short!("mint").into_val(&env), spender.into_val(&env)]);
        assert_eq!(i128::try_from_val(&env, &data).unwrap(), 9);
    }

    #[test]
    fn sep41_client_interface_is_compatible() {
        // The stream contract talks to tokens through soroban_sdk::token::Client
        // (the SEP-41 interface); this token must decode under it.
        let (env, client, admin) = setup();
        let sep41 = soroban_sdk::token::TokenClient::new(&env, &client.address);
        let spender = Address::generate(&env);
        let exp = env.ledger().sequence() + 10;
        sep41.approve(&admin, &spender, &40, &exp);
        assert_eq!(sep41.allowance(&admin, &spender), 40);
        assert_eq!(sep41.decimals(), 7);
        assert_eq!(sep41.balance(&admin), 1_000_000);
        sep41.transfer(&admin, &spender, &5);
        assert_eq!(sep41.balance(&spender), 5);
    }
}
