use anyhow::Result;
use starknet::{
    core::{
        types::{BlockId, BlockTag, FunctionCall},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
};
use starknet_crypto::Felt;
use tracing::info;

pub async fn verify_groth16_proof_onchain(
    rpc_url: &str,
    verifier_address: &str,
    calldata: &Vec<Felt>,
) -> Result<Vec<Felt>> {
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(rpc_url).expect("Invalid RPC URL provided"),
    ));

    let contract_address =
        Felt::from_hex(verifier_address).expect("Invalid verifier address provided");
    info!("contract_address: {:?}", contract_address);

    let result = provider
        .call(
            FunctionCall {
                contract_address,
                entry_point_selector: get_selector_from_name("verify_groth16_proof_bn254").unwrap(),
                calldata: calldata.clone(),
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect("Failed to call contract");

    Ok(result)
}
