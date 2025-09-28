#![deny(unused_crate_dependencies)]

use alloy::{
    providers::{Provider, ProviderBuilder},
    sol,
};
use common::get_env_var;
use eth_rlp_types::BlockHeader;
use eyre::{eyre, Result};
use tokio::time::{sleep, Duration};

// Codegen from embedded Solidity code and precompiled bytecode.
sol! {
    #[allow(missing_docs)]
    // solc v0.8.26; solc Counter.sol --via-ir --optimize --bin
    #[sol(rpc, bytecode="0x6080604052348015600f57600080fd5b5061011e8061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c80639663f88f14602d575b600080fd5b6033604c565b6040805192835260208301919091520160405180910390f35b6000806064431160b05760405162461bcd60e51b815260206004820152602560248201527f426c6f636b206e756d626572206d75737420626520677265617465722074686160448201526406e203130360dc1b606482015260840160405180910390fd5b60b960644360c2565b92834092509050565b8181038181111560e257634e487b7160e01b600052601160045260246000fd5b9291505056fea2646970667358221220b23547ed5542ead2de6260575deb6fc65775cd0f04fd8e5e88ba243c70e5dcb364736f6c634300081a0033")]
    contract BlockHashFetcher {
        function getBlockHash() external view returns (uint256 blockNumber, bytes32 blockHash) {
            require(block.number > 100, "Block number must be greater than 100");
            blockNumber = block.number - 100;
            blockHash = blockhash(blockNumber);
            return (blockNumber, blockHash);
        }
    }
}

#[allow(dead_code)]
pub async fn get_finalized_block_hash() -> Result<(u64, String)> {
    let rpc_url = get_env_var("ETH_RPC_URL")?;
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(1);

    let mut attempts = 0;
    loop {
        attempts += 1;
        let result: Result<(u64, String)> = async {
            let provider = ProviderBuilder::new()
                .connect_anvil_with_wallet_and_config(|anvil| anvil.fork(rpc_url.clone()))
                .map_err(|e| eyre!("Failed to setup Anvil provider: {}", e))?;

            let contract = BlockHashFetcher::deploy(&provider).await?;
            let result = contract.getBlockHash().call().await?;

            let block_number: u64 = result.blockNumber.try_into()?;
            let block_hash = result.blockHash.to_string();

            Ok((block_number, block_hash))
        }
        .await;

        match result {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempts >= MAX_RETRIES {
                    return Err(eyre!("get_finalized_block_hash failed: {}", e));
                }
                tracing::error!(
                    attempts = attempts,
                    max_retries = MAX_RETRIES,
                    error = %e.to_string(),
                    "Attempt failed"
                );
                sleep(RETRY_DELAY).await;
            }
        }
    }
}

#[allow(dead_code)]
pub async fn get_block_by_number(block_number: u64) -> Result<alloy::rpc::types::Block> {
    let rpc_url = get_env_var("ETH_RPC_URL")?;
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(1);

    let mut attempts = 0;
    loop {
        attempts += 1;
        let result: Result<alloy::rpc::types::Block> = async {
            let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);

            let block = provider
                .get_block_by_number(block_number.into())
                .await?
                .ok_or_else(|| eyre!("Block {} not found", block_number))?;

            Ok(block)
        }
        .await;

        match result {
            Ok(block) => return Ok(block),
            Err(e) => {
                if attempts >= MAX_RETRIES {
                    return Err(eyre!("get_block_by_number failed: {}", e));
                }
                tracing::warn!(
                    attempts = attempts,
                    max_retries = MAX_RETRIES,
                    block_number = block_number,
                    error = %e.to_string(),
                    "RPC attempt failed, retrying"
                );
                sleep(RETRY_DELAY).await;
            }
        }
    }
}

#[allow(dead_code)]
pub fn alloy_block_to_block_header(block: &alloy::rpc::types::Block) -> BlockHeader {
    BlockHeader {
        block_hash: format!("0x{}", hex::encode(block.header.hash.as_slice())),
        number: block.header.number as i64,
        gas_limit: block.header.gas_limit as i64,
        gas_used: block.header.gas_used as i64,
        nonce: format!("0x{:016x}", u64::from_be_bytes(block.header.nonce.0)),
        transaction_root: Some(block.header.transactions_root.to_string()),
        receipts_root: Some(block.header.receipts_root.to_string()),
        state_root: Some(block.header.state_root.to_string()),
        base_fee_per_gas: block
            .header
            .base_fee_per_gas
            .map(|fee| format!("0x{:x}", fee)),
        parent_hash: Some(block.header.parent_hash.to_string()),
        ommers_hash: Some(block.header.ommers_hash.to_string()),
        miner: Some(block.header.beneficiary.to_string()),
        logs_bloom: Some(format!(
            "0x{}",
            hex::encode(block.header.logs_bloom.as_slice())
        )),
        difficulty: Some(format!("0x{:x}", block.header.difficulty)),
        totaldifficulty: None, // Not available in single block response
        sha3_uncles: Some(block.header.ommers_hash.to_string()),
        timestamp: Some(format!("0x{:x}", block.header.timestamp)),
        extra_data: Some(format!("0x{}", hex::encode(&block.header.extra_data))),
        mix_hash: Some(block.header.mix_hash.to_string()),
        withdrawals_root: block.header.withdrawals_root.map(|root| root.to_string()),
        blob_gas_used: block.header.blob_gas_used.map(|gas| format!("0x{:x}", gas)),
        excess_blob_gas: block
            .header
            .excess_blob_gas
            .map(|gas| format!("0x{:x}", gas)),
        parent_beacon_block_root: block
            .header
            .parent_beacon_block_root
            .map(|root| root.to_string()),
        request_hash: block.header.requests_hash.map(|hash| hash.to_string()),
    }
}
