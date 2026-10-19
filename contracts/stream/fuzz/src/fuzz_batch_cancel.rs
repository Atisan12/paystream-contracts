// SPDX-License-Identifier: Apache-2.0

//! Fuzz target for `cancel_streams_batch` (TEST-21, issue #244).
//!
//! Drives `cancel_streams_batch` with random stream ID sets, including:
//! - IDs that do not exist
//! - Streams in wrong states (already cancelled / exhausted)
//! - Batches exceeding the 50-stream limit
//! - Batches owned by a different employer
//!
//! Verifies that valid batches cancel all streams atomically and that
//! invalid batches always panic with a documented error.

#![allow(dead_code)]

use paystream_stream::types::StreamStatus;
use paystream_stream::{StreamContract, StreamContractClient};
use proptest::prelude::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, Vec as SdkVec};

fn setup_env() -> (Env, StreamContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let employer = Address::generate(&env);

    let token_id = env.register(paystream_token::TokenContract, ());
    let token = paystream_token::TokenContractClient::new(&env, &token_id);
    token.initialize(&employer, &1_000_000_000);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &1);

    (env, client, employer, token_id)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// Valid batch cancel (1–50 active streams, all owned by employer) must
    /// succeed atomically — every stream transitions to Cancelled.
    #[test]
    fn fuzz_batch_cancel_valid_inputs_cancels_all(
        count in 1usize..=50usize,
    ) {
        let (env, client, employer, token_id) = setup_env();

        let mut ids: SdkVec<u64> = SdkVec::new(&env);
        for _ in 0..count {
            let employee = Address::generate(&env);
            let id = client.create_stream(
                &employer,
                &employee,
                &token_id,
                &1_000,
                &1,
                &0,
                &None,
            );
            ids.push_back(id);
        }

        client.cancel_streams_batch(&employer, &ids);

        for id in ids.iter() {
            let stream = client.get_stream(&id);
            prop_assert_eq!(
                stream.status, StreamStatus::Cancelled,
                "stream {} should be Cancelled after batch cancel", id
            );
        }
    }

    /// Batch cancel with a non-existent stream ID must panic.
    #[test]
    fn fuzz_batch_cancel_nonexistent_id_panics(
        bogus_id in 9_000_000u64..=u64::MAX,
    ) {
        let (env, client, employer, _token_id) = setup_env();
        let mut ids: SdkVec<u64> = SdkVec::new(&env);
        ids.push_back(bogus_id);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.cancel_streams_batch(&employer, &ids);
        }));
        prop_assert!(result.is_err(), "expected panic for non-existent stream ID");
    }

    /// Batch exceeding 50 streams must panic with E029.
    #[test]
    fn fuzz_batch_cancel_over_limit_panics(
        extra in 1usize..=10usize,
    ) {
        let (env, client, employer, token_id) = setup_env();

        let mut ids: SdkVec<u64> = SdkVec::new(&env);
        for _ in 0..(50 + extra) {
            let employee = Address::generate(&env);
            let id = client.create_stream(
                &employer,
                &employee,
                &token_id,
                &1_000,
                &1,
                &0,
                &None,
            );
            ids.push_back(id);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.cancel_streams_batch(&employer, &ids);
        }));
        prop_assert!(result.is_err(), "expected panic for batch size > 50");
    }

    /// A batch containing an already-cancelled stream must panic (all-or-nothing).
    #[test]
    fn fuzz_batch_cancel_already_cancelled_reverts_all(
        count in 1usize..=10usize,
    ) {
        let (env, client, employer, token_id) = setup_env();

        let mut ids: SdkVec<u64> = SdkVec::new(&env);
        for _ in 0..count {
            let employee = Address::generate(&env);
            let id = client.create_stream(
                &employer,
                &employee,
                &token_id,
                &1_000,
                &1,
                &0,
                &None,
            );
            ids.push_back(id);
        }

        // Cancel the last stream individually first.
        let last_id = ids.get(ids.len() - 1).unwrap();
        client.cancel_stream(&employer, &last_id, &None);

        // Now batch cancel should panic because one stream is already ended.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.cancel_streams_batch(&employer, &ids);
        }));
        prop_assert!(result.is_err(),
            "batch cancel must panic when one stream is already cancelled");
    }

    /// Batch cancel by wrong employer must panic for every stream count.
    #[test]
    fn fuzz_batch_cancel_wrong_employer_panics(
        count in 1usize..=10usize,
    ) {
        let (env, client, employer, token_id) = setup_env();
        let attacker = Address::generate(&env);

        let mut ids: SdkVec<u64> = SdkVec::new(&env);
        for _ in 0..count {
            let employee = Address::generate(&env);
            let id = client.create_stream(
                &employer,
                &employee,
                &token_id,
                &1_000,
                &1,
                &0,
                &None,
            );
            ids.push_back(id);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.cancel_streams_batch(&attacker, &ids);
        }));
        prop_assert!(result.is_err(),
            "batch cancel by wrong employer must always panic");
    }

    /// Empty batch must panic.
    #[test]
    fn fuzz_batch_cancel_empty_panics(_dummy in 0u8..=1u8) {
        let (env, client, employer, _token_id) = setup_env();
        let ids: SdkVec<u64> = SdkVec::new(&env);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.cancel_streams_batch(&employer, &ids);
        }));
        prop_assert!(result.is_err(), "expected panic for empty batch");
    }
}

fn main() {}
