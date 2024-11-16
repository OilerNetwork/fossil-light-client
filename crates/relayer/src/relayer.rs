use alloy::{
    network::EthereumWallet, primitives::U256, providers::ProviderBuilder,
    signers::local::PrivateKeySigner, sol_types::sol,
};
use common::{get_env_var, get_var};
use eyre::Result;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum RelayerError {
    #[error("Ethereum provider initialization failed: {0}")]
    ProviderError(String),

    #[error("Transaction failed: {0}")]
    TransactionError(String),
}

sol!(
    #[sol(rpc)]
    L1MessagesSender,
    "abi/L1MessagesSender.json"
);

pub struct Relayer {
    wallet: EthereumWallet,
    l2_recipient_addr: U256,
}

impl Relayer {
    pub async fn new() -> Result<Self> {
        // Load the private key and initialize the signer
        let signer: PrivateKeySigner = get_var("ACCOUNT_PRIVATE_KEY")?;

        // Create the wallet
        let wallet = EthereumWallet::from(signer.clone());

        let l2_recipient_addr: U256 = get_var("L2_MSG_PROXY")?;
        info!("Using L2 recipient address: {:?}", l2_recipient_addr);

        Ok(Self {
            wallet,
            l2_recipient_addr,
        })
    }

    pub async fn send_finalized_block_hash_to_l2(&self) -> Result<()> {
        // Create the provider
        let provider_url = get_env_var("ANVIL_URL")?;

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(self.wallet.clone())
            .on_builtin(&provider_url)
            .await
            .map_err(|e| RelayerError::ProviderError(e.to_string()))?;
        info!("Connected to Ethereum provider at {}", provider_url);

        // Load the contract address and initialize the contract
        let address = get_var("L1_MESSAGE_SENDER")?;

        let contract = L1MessagesSender::new(address, &provider);
        info!(
            "Initialized L1MessagesSender contract at address {}",
            address
        );

        // Prepare and send the transaction
        let call_builder = contract
            .sendFinalizedBlockHashToL2(self.l2_recipient_addr)
            .value(U256::from(30000));
        info!("Prepared transaction to send block hash with value: 30000 Wei");
        info!(
            "Sending transaction to L2 address: {:?}",
            self.l2_recipient_addr
        );

        let pending_tx = call_builder
            .send()
            .await
            .map_err(|e| RelayerError::TransactionError(e.to_string()))?;
        info!(
            "Transaction sent successfully. Tx hash: {:?}",
            pending_tx.tx_hash()
        );

        Ok(())
    }
}
