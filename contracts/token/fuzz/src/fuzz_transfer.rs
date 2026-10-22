// SPDX-License-Identifier: Apache-2.0

//! Fuzz target for `transfer` (TOK-19).
//!
//! Drives random sequences of transfers between a fixed set of accounts with
//! amounts across the whole i128 range (including 0, negatives and values far
//! above any balance) and checks after every step that:
//!
//! * a transfer succeeds iff `0 < amount <= balance(from)`;
//! * on success, exactly `amount` moves from `from` to `to` (self-transfers
//!   leave the balance unchanged);
//! * on failure, no balance changes;
//! * total supply always equals the sum of balances and no balance goes negative.

mod common;

use common::{config, Harness, ACCOUNTS};
use proptest::prelude::*;

fn amount() -> impl Strategy<Value = i128> {
    prop_oneof![
        0i128..=2_000,
        Just(0i128),
        Just(-1i128),
        Just(i128::MIN),
        Just(i128::MAX),
        any::<i128>(),
    ]
}

proptest! {
    #![proptest_config(config(256))]

    #[test]
    fn fuzz_transfer_sequences(
        supply in 0i128..=1_000_000_000_000i128,
        steps in proptest::collection::vec((0..ACCOUNTS, 0..ACCOUNTS, amount()), 1..24),
    ) {
        let h = Harness::new(supply);
        for (from_i, to_i, amount) in steps {
            let (from, to) = (&h.accounts[from_i], &h.accounts[to_i]);
            let before = h.balances();
            let res = h.client.try_transfer(from, to, &amount);
            let after = h.balances();

            let should_succeed = amount > 0 && amount <= before[from_i];
            prop_assert_eq!(res.is_ok(), should_succeed,
                "transfer({}, {}, {}) with balance {}", from_i, to_i, amount, before[from_i]);
            if should_succeed && from_i != to_i {
                prop_assert_eq!(after[from_i], before[from_i] - amount);
                prop_assert_eq!(after[to_i], before[to_i] + amount);
            } else {
                prop_assert_eq!(&after, &before, "failed or self transfer must not change balances");
            }
            h.assert_supply_invariant();
        }
    }

    /// Balances near i128::MAX: a recipient holding almost the whole supply
    /// must be credited exactly (no wrap). Because balances always sum to the
    /// supply (<= i128::MAX), the T001 overflow branch must never trigger on a
    /// transfer; if it did, the credit must fail and leave state untouched.
    #[test]
    fn fuzz_transfer_recipient_near_i128_max(amount in 1i128..=2_000) {
        let h = Harness::new(i128::MAX - 2_000);
        let (admin, rich, poor) = (h.admin().clone(), h.accounts[1].clone(), h.accounts[2].clone());
        h.client.transfer(&admin, &rich, &(i128::MAX - 2_000 - 2_000));
        h.client.transfer(&admin, &poor, &2_000);
        let rich_before = h.client.balance(&rich);

        let res = h.client.try_transfer(&poor, &rich, &amount);
        match rich_before.checked_add(amount) {
            Some(expected) => {
                prop_assert!(res.is_ok());
                prop_assert_eq!(h.client.balance(&rich), expected);
            }
            None => {
                prop_assert!(res.is_err(), "overflowing credit must fail");
                prop_assert_eq!(h.client.balance(&rich), rich_before);
            }
        }
        h.assert_supply_invariant();
    }
}

fn main() {}
