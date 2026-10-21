// SPDX-License-Identifier: Apache-2.0

use soroban_sdk::{contracttype, Address, String};

#[contracttype]
pub enum TokenDataKey {
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
    Admin,
    PendingAdmin,
    AdminNonce,
    /// Token metadata (SEP-41 `decimals` / `name` / `symbol`).
    Metadata,
}

/// SEP-41 token metadata.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TokenMetadata {
    pub decimal: u32,
    pub name: String,
    pub symbol: String,
}

/// SEP-41 `transfer` event data when the recipient is a muxed address.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TransferMuxedData {
    pub amount: i128,
    pub to_muxed_id: u64,
}
