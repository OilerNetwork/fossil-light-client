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
        ipfs_hash: Option<String>,
    ) -> Result<Felt, StarknetHandlerError> {
        println!("proof: {:?}", &proof[..8.min(proof.len())]);
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

        let mut calldata = vec![];
        let mut hash_calldata = vec![];

        proof.encode(&mut calldata)?;

        match ipfs_hash {
            Some(hash) => {
                Option::Some(ByteArray::from(hash.as_str())).encode(&mut hash_calldata)?
            }
            None => Option::<ByteArray>::None.encode(&mut hash_calldata)?,
        };
        calldata.extend(hash_calldata);

        println!("calldata: {:?}", &calldata[..8.min(calldata.len())]);

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

            match self.account.execute_v1(vec![call.clone()]).send().await {
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
