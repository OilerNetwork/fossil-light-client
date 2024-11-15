use eyre::Result;
use starknet::{
    accounts::Account,
    core::{
        types::{BlockId, BlockTag, Call, FunctionCall},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
};
use starknet_crypto::Felt;
use tracing::info;

use dotenv::dotenv;
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::chain_id,
    signers::{LocalWallet, SigningKey},
};

pub struct StarknetProvider {
    pub provider: JsonRpcClient<HttpTransport>,
}

impl StarknetProvider {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            provider: JsonRpcClient::new(HttpTransport::new(
                Url::parse(rpc_url).expect("Invalid RPC URL provided"),
            )),
        }
    }

    pub async fn verify_groth16_proof_onchain(
        &self,
        verifier_address: &str,
        calldata: &Vec<Felt>,
    ) -> Result<Vec<Felt>> {
        let contract_address =
            Felt::from_hex(verifier_address).expect("Invalid verifier address provided");
        info!("contract_address: {:?}", contract_address);

        let result = self
            .provider
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector: get_selector_from_name("verify_groth16_proof_bn254")
                        .unwrap(),
                    calldata: calldata.clone(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect("Failed to call contract");

        Ok(result)
    }
}

#[derive(Debug)]
pub struct StarknetAccount {
    pub account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl StarknetAccount {
    pub fn new(
        provider: JsonRpcClient<HttpTransport>,
        account_private_key: &str,
        account_address: &str,
    ) -> Self {
        dotenv().ok();

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&account_private_key).expect("Invalid private key provided"),
        ));

        Self {
            account: SingleOwnerAccount::new(
                provider,
                signer,
                Felt::from_hex(&account_address).expect("Invalid address provided"),
                chain_id::SEPOLIA,
                ExecutionEncoding::New,
            ),
        }
    }

    pub async fn update_mmr_state(
        &self,
        store_address: Felt,
        latest_block_number: u64,
        new_mmr_root: Felt,
    ) -> Result<Felt> {
        let tx = self
            .account
            .execute_v1(vec![Call {
                selector: get_selector_from_name("store_mmr_root").unwrap(),
                calldata: vec![Felt::from(latest_block_number), new_mmr_root],
                to: store_address,
            }])
            .send()
            .await?;

        Ok(tx.transaction_hash)
    }
}
