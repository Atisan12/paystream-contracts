// SPDX-License-Identifier: Apache-2.0

//! Shared harness for the token fuzz targets (TOK-19).
//!
//! Every case gets a fresh `Env` with all auths mocked, a deployed token and a
//! small fixed set of accounts, so cases are independent and reproducible.

#![allow(dead_code)]

use paystream_token::{TokenContract, TokenContractClient};
use proptest::test_runner::Config as ProptestConfig;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

/// Number of accounts per case (index 0 is the admin).
pub const ACCOUNTS: usize = 4;

pub struct Harness {
    pub env: Env,
    pub client: TokenContractClient<'static>,
    pub accounts: std::vec::Vec<Address>,
}

impl Harness {
    /// Deploy the token and mint `initial_supply` to the admin (`accounts[0]`).
    pub fn new(initial_supply: i128) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(TokenContract, ());
        let client = TokenContractClient::new(&env, &id);
        let accounts: std::vec::Vec<Address> =
            (0..ACCOUNTS).map(|_| Address::generate(&env)).collect();
        client.initialize(&accounts[0], &initial_supply);
        Harness {
            env,
            client,
            accounts,
        }
    }

    pub fn admin(&self) -> &Address {
        &self.accounts[0]
    }

    pub fn balances(&self) -> std::vec::Vec<i128> {
        self.accounts
            .iter()
            .map(|a| self.client.balance(a))
            .collect()
    }

    /// Sum of all tracked balances, `None` on i128 overflow.
    pub fn balance_sum(&self) -> Option<i128> {
        self.balances()
            .into_iter()
            .try_fold(0i128, |acc, b| acc.checked_add(b))
    }

    /// Supply invariant: total supply equals the sum of balances and nothing is negative.
    pub fn assert_supply_invariant(&self) {
        let balances = self.balances();
        assert!(
            balances.iter().all(|b| *b >= 0),
            "negative balance: {balances:?}"
        );
        assert_eq!(
            self.balance_sum(),
            Some(self.client.total_supply()),
            "total_supply must equal the sum of balances: {balances:?}"
        );
    }
}

/// Proptest config honouring `PROPTEST_CASES` (CI sets it so each target fits
/// its time box), defaulting to `default_cases`.
pub fn config(default_cases: u32) -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default_cases);
    ProptestConfig {
        cases,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}
