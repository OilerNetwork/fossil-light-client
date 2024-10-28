use std::env;
use std::str::FromStr;

use alloy_network::EthereumWallet;
use alloy_primitives::Uint;
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::sol;


sol!(
    #[sol(rpc)]
    L1MessagesSender,
    "L1MessagesSender.json"
);


fn raw_var(name: &str) -> String {
    return env::var(name).expect(&format!("Missing environment variable: {name}. Try exporting variables: export $(grep -v '^#' ../fossil/ethereum/anvil.env | xargs)"))
}

fn get_var<T: FromStr>(name: &str) -> T where <T as FromStr>::Err: std::fmt::Debug {
    return raw_var(name).parse().expect(&format!("Unable to parse {} environment variable.", name))
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer: PrivateKeySigner = get_var("ACCOUNT_PRIVATE_KEY");
    let wallet = EthereumWallet::from(signer);
    
    let provider = ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_builtin(&raw_var("ETH_RPC_URL")).await?;
    
    
    let address = get_var("L1_MESSAGE_SENDER_ADDRESS");
    let contract = L1MessagesSender::new(address, &provider);

    let call_builder = contract.sendLatestParentHashToL2().value(Uint::from(1));

    let _pending_tx = call_builder.send().await?;

    Ok(())
}
