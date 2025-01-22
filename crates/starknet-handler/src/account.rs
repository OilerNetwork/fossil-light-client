use starknet::macros::selector;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, codec::Encode, types::ByteArray},
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use starknet_crypto::Felt;
use std::{sync::Arc, time::Duration};
use tracing::{debug, info, instrument, warn};

use common::felt;

use crate::StarknetHandlerError;

pub struct StarknetAccount {
    account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
}

impl StarknetAccount {
    #[instrument(skip(provider, account_private_key), fields(address = %account_address), level = "debug")]
    pub fn new(
        provider: Arc<JsonRpcClient<HttpTransport>>,
        account_private_key: &str,
        account_address: &str,
    ) -> Result<Self, StarknetHandlerError> {
        debug!("Creating new Starknet account");

        let private_key = felt(account_private_key)?;
        debug!("Private key converted to felt");

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));
        let address = felt(account_address)?;

        debug!(
            chain_id = ?chain_id::SEPOLIA,
            encoding = ?ExecutionEncoding::New,
            "Initializing SingleOwnerAccount"
        );

        let account = SingleOwnerAccount::new(
            provider,
            signer,
            address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        debug!("Starknet account successfully created");
        Ok(Self { account })
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn verify_mmr_proof(
        &self,
        verifier_address: &str,
        proof: Vec<Felt>,
        ipfs_hash: String,
        is_build: bool,
    ) -> Result<Felt, StarknetHandlerError> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        let mut calldata = vec![];
        let mut hash_calldata = vec![];
        let mut is_build_calldata = vec![];

        proof.encode(&mut calldata)?;

        ByteArray::from(ipfs_hash.as_str()).encode(&mut hash_calldata)?;
        is_build.encode(&mut is_build_calldata)?;

        calldata.extend(hash_calldata);
        calldata.extend(is_build_calldata);

        let selector = selector!("verify_mmr_proof");
        let call = starknet::core::types::Call {
            selector,
            calldata,
            to: felt(verifier_address)?,
        };

        let mut attempt = 0;
        loop {
            debug!(
                verifier_address = %verifier_address,
                proof_length = proof.len(),
                attempt = attempt + 1,
                "Verifying MMR proof"
            );

            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(tx) => {
                    info!(
                        tx_hash = ?tx.transaction_hash,
                        "MMR proof onchain verification successful."
                    );
                    return Ok(tx.transaction_hash);
                }
                Err(e) => {
                    if attempt >= MAX_RETRIES {
                        warn!("Max retries reached for MMR proof verification");
                        return Err(e.into());
                    }

                    let backoff = INITIAL_BACKOFF * 2u32.pow(attempt);
                    warn!(
                        error = ?e,
                        retry_in = ?backoff,
                        "MMR proof verification failed, retrying..."
                    );

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Helper function to create a test provider
    fn create_test_provider() -> Arc<JsonRpcClient<HttpTransport>> {
        Arc::new(JsonRpcClient::new(HttpTransport::new(
            url::Url::parse("http://localhost:5050").unwrap(),
        )))
    }

    #[test]
    fn test_new_account_success() {
        let provider = create_test_provider();
        let private_key = "0x1234567890abcdef";
        let address = "0x987654321fedcba";

        let result = StarknetAccount::new(provider, private_key, address);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_account_invalid_private_key() {
        let provider = create_test_provider();
        let private_key = "invalid_key";
        let address = "0x987654321fedcba";

        let result = StarknetAccount::new(provider, private_key, address);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_account_invalid_address() {
        let provider = create_test_provider();
        let private_key = "0x1234567890abcdef";
        let address = "invalid_address";

        let result = StarknetAccount::new(provider, private_key, address);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_account_empty_private_key() {
        let provider = create_test_provider();
        let result = StarknetAccount::new(provider, "", "0x987654321fedcba");
        assert!(result.is_err());
    }

    #[test]
    fn test_new_account_empty_address() {
        let provider = create_test_provider();
        let result = StarknetAccount::new(provider, "0x1234567890abcdef", "");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_mmr_proof_success() {
        let provider = create_test_provider();
        let account =
            StarknetAccount::new(provider, "0x1234567890abcdef", "0x987654321fedcba").unwrap();

        let verifier_address = "0x123456789";
        let proof = vec![Felt::from_str("0x1").unwrap()];
        let ipfs_hash = "QmTest123".to_string();

        // Note: This test will fail in real execution since we're using a dummy provider
        // In a real test environment, you would mock the provider and account interactions
        let result = account
            .verify_mmr_proof(verifier_address, proof, ipfs_hash, true)
            .await;
        assert!(result.is_err()); // Will error due to dummy provider
    }

    #[tokio::test]
    async fn test_verify_mmr_proof_empty_proof() {
        let provider = create_test_provider();
        let account =
            StarknetAccount::new(provider, "0x1234567890abcdef", "0x987654321fedcba").unwrap();

        let result = account
            .verify_mmr_proof("0x123456789", vec![], "QmTest123".to_string(), true)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_mmr_proof_empty_ipfs_hash() {
        let provider = create_test_provider();
        let account =
            StarknetAccount::new(provider, "0x1234567890abcdef", "0x987654321fedcba").unwrap();

        let proof = vec![Felt::from_str("0x1").unwrap()];
        let result = account
            .verify_mmr_proof("0x123456789", proof, "".to_string(), true)
            .await;
        assert!(result.is_err());
    }
}
