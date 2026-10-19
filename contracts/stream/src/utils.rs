// SPDX-License-Identifier: Apache-2.0

use soroban_sdk::{Address, Env};

/// Get the native XLM SAC (Stellar Asset Contract) address.
///
/// Native XLM on Stellar is wrapped as a SAC with a specific address derived
/// from the native asset ID. This utility looks up the native XLM SAC address
/// programmatically without hardcoding it, making it portable across networks.
///
/// # Parameters
/// - `env` — Soroban environment
///
/// # Returns
/// The `Address` of the native XLM SAC contract.
///
/// # Example
/// ```ignore
/// let native_xlm = get_native_xlm_sac(&env);
/// let token_client = token::Client::new(&env, &native_xlm);
/// ```
pub fn get_native_xlm_sac(env: &Env) -> Address {
    Address::from_contract_id(
        env,
        &soroban_sdk::token::StellarAssetClient::native(env).contract_id(),
    )
}

#[cfg(test)]
mod tests {
    use soroban_sdk::Env;

    use super::*;

    #[test]
    fn test_get_native_xlm_sac_returns_valid_address() {
        let env = Env::default();
        let native_xlm = get_native_xlm_sac(&env);
        // Verify it returns an Address (any valid address from StellarAssetClient::native)
        // The function should not panic and return a valid contract address.
        // On-chain, this would resolve to the actual native XLM SAC address.
        let _client = soroban_sdk::token::Client::new(&env, &native_xlm);
        // If we got here, the address is valid and usable
    }
}
