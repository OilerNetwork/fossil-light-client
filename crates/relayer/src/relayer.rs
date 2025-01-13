use alloy::{
    network::EthereumWallet,
    primitives::U256,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol_types::sol,
    transports::{RpcError, TransportErrorKind},
};
use common::{get_env_var, get_var, UtilsError};
// use eyre::Result;
use std::time::Duration;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum RelayerError {
    #[error("Utils error: {0}")]
    Utils(#[from] UtilsError),
    #[error("RPC error: {0}")]
    RpcError(#[from] RpcError<TransportErrorKind>),
    #[error("Alloy contract error: {0}")]
    AlloyContract(#[from] alloy_contract::Error),
    #[error("Pending transaction error: {0}")]
    PendingTransaction(#[from] alloy::providers::PendingTransactionError),
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
    pub async fn new() -> Result<Self, RelayerError> {
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

    pub async fn send_finalized_block_hash_to_l2(&self) -> Result<(), RelayerError> {
        // Create the provider
        let provider_url = get_env_var("ETH_RPC_URL")?;

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(self.wallet.clone())
            .on_builtin(&provider_url)
            .await?;
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

        let pending_tx = call_builder.send().await?;
        let tx_hash = pending_tx
            .with_required_confirmations(1)
            .with_timeout(Some(Duration::from_secs(60)))
            .watch()
            .await?;
        info!("Transaction confirmed successfully. Tx hash: {:?}", tx_hash);

        Ok(())
    }
}
