// SPDX-License-Identifier: Apache-2.0

//! Fuzz target for `approve` / `transfer_from` / `burn_from` (TOK-19).
//!
//! Interleaves random approvals, spends, burns and ledger advances, tracking a
//! reference model of every (owner, spender) allowance. After every step:
//!
//! * `allowance()` equals the model (0 once expired), never negative;
//! * a spend succeeds iff `0 < amount <= allowance` (unexpired) and
//!   `amount <= balance(owner)`, and then reduces the allowance by exactly
//!   `amount`;
//! * negative approvals and negative/zero spends are rejected without effect;
//! * total supply equals the sum of balances.

mod common;

use common::{config, Harness, ACCOUNTS};
use proptest::prelude::*;
use soroban_sdk::testutils::Ledger;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
enum Op {
    Approve {
        owner: usize,
        spender: usize,
        amount: i128,
        ttl: u32,
    },
    TransferFrom {
        spender: usize,
        owner: usize,
        to: usize,
        amount: i128,
    },
    BurnFrom {
        spender: usize,
        owner: usize,
        amount: i128,
    },
    Advance {
        ledgers: u32,
    },
}

fn amount() -> impl Strategy<Value = i128> {
    prop_oneof![
        0i128..=5_000,
        Just(-1i128),
        Just(i128::MIN),
        Just(i128::MAX),
        any::<i128>()
    ]
}

fn op() -> impl Strategy<Value = Op> {
    let idx = 0..ACCOUNTS;
    prop_oneof![
        (idx.clone(), idx.clone(), amount(), 0u32..20).prop_map(|(owner, spender, amount, ttl)| {
            Op::Approve {
                owner,
                spender,
                amount,
                ttl,
            }
        }),
        (idx.clone(), idx.clone(), idx.clone(), amount()).prop_map(
            |(spender, owner, to, amount)| Op::TransferFrom {
                spender,
                owner,
                to,
                amount
            }
        ),
        (idx.clone(), idx.clone(), amount()).prop_map(|(spender, owner, amount)| Op::BurnFrom {
            spender,
            owner,
            amount
        }),
        (1u32..10).prop_map(|ledgers| Op::Advance { ledgers }),
    ]
}

proptest! {
    #![proptest_config(config(256))]

    #[test]
    fn fuzz_allowance_sequences(ops in proptest::collection::vec(op(), 1..32)) {
        let h = Harness::new(10_000);
        // Spread the supply so every account can be an owner.
        for i in 1..ACCOUNTS {
            h.client.transfer(h.admin(), &h.accounts[i], &2_000);
        }
        // (owner, spender) -> (amount, expiration_ledger)
        let mut model: BTreeMap<(usize, usize), (i128, u32)> = BTreeMap::new();

        for op in ops {
            let seq = h.env.ledger().sequence();
            match op {
                Op::Approve { owner, spender, amount, ttl } => {
                    let exp = seq + ttl;
                    let res = h.client.try_approve(&h.accounts[owner], &h.accounts[spender], &amount, &exp);
                    prop_assert_eq!(res.is_ok(), amount >= 0, "approve({})", amount);
                    if amount >= 0 {
                        model.insert((owner, spender), (amount, exp));
                    }
                }
                Op::TransferFrom { spender, owner, to, amount } => {
                    let before = h.balances();
                    let (allowed, exp) = model.get(&(owner, spender)).copied().unwrap_or((0, 0));
                    let ok_expected = amount > 0 && seq <= exp && amount <= allowed && amount <= before[owner];
                    let res = h.client.try_transfer_from(&h.accounts[spender], &h.accounts[owner], &h.accounts[to], &amount);
                    prop_assert_eq!(res.is_ok(), ok_expected,
                        "transfer_from amount={} allowed={} exp={} seq={} bal={}", amount, allowed, exp, seq, before[owner]);
                    if ok_expected {
                        model.insert((owner, spender), (allowed - amount, exp));
                    } else {
                        prop_assert_eq!(h.balances(), before, "failed transfer_from must not move funds");
                    }
                }
                Op::BurnFrom { spender, owner, amount } => {
                    let before = h.balances();
                    let (allowed, exp) = model.get(&(owner, spender)).copied().unwrap_or((0, 0));
                    let ok_expected = amount > 0 && seq <= exp && amount <= allowed && amount <= before[owner];
                    let res = h.client.try_burn_from(&h.accounts[spender], &h.accounts[owner], &amount);
                    prop_assert_eq!(res.is_ok(), ok_expected, "burn_from amount={} allowed={}", amount, allowed);
                    if ok_expected {
                        model.insert((owner, spender), (allowed - amount, exp));
                    } else {
                        prop_assert_eq!(h.balances(), before);
                    }
                }
                Op::Advance { ledgers } => {
                    h.env.ledger().with_mut(|l| l.sequence_number += ledgers);
                }
            }

            // Allowance view matches the model and is never negative.
            let seq = h.env.ledger().sequence();
            for owner in 0..ACCOUNTS {
                for spender in 0..ACCOUNTS {
                    let (amt, exp) = model.get(&(owner, spender)).copied().unwrap_or((0, 0));
                    let expected = if seq > exp { 0 } else { amt };
                    let got = h.client.allowance(&h.accounts[owner], &h.accounts[spender]);
                    prop_assert!(got >= 0);
                    prop_assert_eq!(got, expected, "allowance({}, {})", owner, spender);
                }
            }
            h.assert_supply_invariant();
        }
    }
}

fn main() {}
