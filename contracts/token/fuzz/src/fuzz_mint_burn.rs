// SPDX-License-Identifier: Apache-2.0

//! Fuzz target for `mint` / `burn` near i128::MAX (TOK-19).
//!
//! Starts from an initial supply anywhere up to i128::MAX and applies random
//! mints and burns, with amounts biased towards the overflow boundary. After
//! every step:
//!
//! * a mint succeeds iff `amount > 0` and neither the recipient balance nor
//!   the total supply overflows (T001 otherwise, with no state change);
//! * a burn succeeds iff `0 < amount <= balance`;
//! * the admin nonce advances only on successful mints;
//! * total supply equals the sum of balances and never goes negative.

mod common;

use common::{config, Harness, ACCOUNTS};
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Op {
    Mint { to: usize, amount: i128 },
    Burn { from: usize, amount: i128 },
}

fn near_max() -> impl Strategy<Value = i128> {
    prop_oneof![
        Just(i128::MAX),
        (0i128..=1_000).prop_map(|d| i128::MAX - d),
        (0i128..=1_000).prop_map(|d| i128::MAX / 2 - d),
        1i128..=1_000_000,
        Just(0i128),
        Just(-1i128),
        Just(i128::MIN),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..ACCOUNTS, near_max()).prop_map(|(to, amount)| Op::Mint { to, amount }),
        (0..ACCOUNTS, near_max()).prop_map(|(from, amount)| Op::Burn { from, amount }),
    ]
}

proptest! {
    #![proptest_config(config(256))]

    #[test]
    fn fuzz_mint_burn_near_i128_max(
        initial in prop_oneof![Just(0i128), Just(i128::MAX), 0i128..=i128::MAX],
        ops in proptest::collection::vec(op(), 1..24),
    ) {
        let h = Harness::new(initial);
        let admin = h.admin().clone();

        for op in ops {
            let before = h.balances();
            let supply = h.client.total_supply();
            let nonce = h.client.admin_nonce();
            match op {
                Op::Mint { to, amount } => {
                    let ok_expected = amount > 0
                        && supply.checked_add(amount).is_some()
                        && before[to].checked_add(amount).is_some();
                    let res = h.client.try_mint(&admin, &h.accounts[to], &amount, &nonce);
                    prop_assert_eq!(res.is_ok(), ok_expected,
                        "mint amount={} supply={} bal={}", amount, supply, before[to]);
                    if ok_expected {
                        prop_assert_eq!(h.client.balance(&h.accounts[to]), before[to] + amount);
                        prop_assert_eq!(h.client.total_supply(), supply + amount);
                        prop_assert_eq!(h.client.admin_nonce(), nonce + 1);
                    } else {
                        prop_assert_eq!(h.balances(), before);
                        prop_assert_eq!(h.client.total_supply(), supply);
                        prop_assert_eq!(h.client.admin_nonce(), nonce, "failed mint must not consume the nonce");
                    }
                }
                Op::Burn { from, amount } => {
                    let ok_expected = amount > 0 && amount <= before[from];
                    let res = h.client.try_burn(&h.accounts[from], &amount);
                    prop_assert_eq!(res.is_ok(), ok_expected, "burn amount={} bal={}", amount, before[from]);
                    if ok_expected {
                        prop_assert_eq!(h.client.total_supply(), supply - amount);
                    } else {
                        prop_assert_eq!(h.balances(), before);
                    }
                }
            }
            prop_assert!(h.client.total_supply() >= 0);
            h.assert_supply_invariant();
        }
    }
}

fn main() {}
