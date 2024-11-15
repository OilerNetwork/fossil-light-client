use alloy::{
    network::EthereumWallet, primitives::U256, providers::ProviderBuilder,
    signers::local::PrivateKeySigner, sol_types::sol,
};
use dotenv::dotenv;
use std::{env, str::FromStr};
use tracing::info;

sol!(
    #[sol(rpc)]
    L1MessagesSender,
    "abi/L1MessagesSender.json"
);

fn raw_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("Missing environment variable: {name}"))
}

fn get_var<T: FromStr>(name: &str) -> T
where
    <T as FromStr>::Err: std::fmt::Debug,
{
    raw_var(name)
        .parse()
        .unwrap_or_else(|_| panic!("Unable to parse {} environment variable.", name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("Starting the relayer...");

    // Load the private key and initialize the signer
    let signer: PrivateKeySigner = get_var("ACCOUNT_PRIVATE_KEY");
    info!("Loaded signer from the private key.");

    // Create the wallet and provider
    let wallet = EthereumWallet::from(signer);
    let provider_url = raw_var("ANVIL_URL");
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(&provider_url)
        .await?;
    info!("Connected to Ethereum provider at {}", provider_url);

    // Load the contract address and initialize the contract
    let address = get_var("L1_MESSAGE_SENDER");
    let contract = L1MessagesSender::new(address, &provider);
    info!(
        "Initialized L1MessagesSender contract at address {}",
        address
    );

    // Get the L2 recipient address
    let l2_recipient_addr: U256 = get_var("L2_MSG_PROXY");
    info!("Using L2 recipient address: {:?}", l2_recipient_addr);

    // Prepare and send the transaction
    let call_builder = contract
        .sendFinalizedBlockHashToL2(l2_recipient_addr)
        .value(U256::from(30000));
    info!("Prepared transaction to send block hash with value: 30000 Wei");

    let pending_tx = call_builder.send().await?;
    info!(
        "Transaction sent successfully. Tx hash: {:?}",
        pending_tx.tx_hash()
    );

    Ok(())
}
