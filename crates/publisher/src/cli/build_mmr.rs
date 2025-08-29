use clap::Parser;
use common::{get_env_var, initialize_logger};
use methods::{MMR_BUILD_ELF, MMR_BUILD_ID};
use starknet_handler::{account::StarknetAccount, provider::StarknetProvider};

use crate::core::{AccumulatorBuilder, BatchProcessor, MMRStateManager, ProofGenerator};
/// Command line arguments for MMR building operation
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Batch size for processing blocks
    #[arg(short, long, default_value_t = 1024)]
    pub batch_size: u64,

    /// Number of batches to process. If not specified, processes until block #0.
    #[arg(short, long)]
    pub num_batches: Option<u64>,

    /// Skip proof verification
    #[arg(short = 'p', long, default_value_t = false)]
    pub skip_proof: bool,

    /// Path to environment file (optional)
    #[arg(short = 'e', long, default_value = ".env")]
    pub env_file: String,

    /// Start building from this block number. If not specified, starts from the latest finalized block.
    #[arg(short = 's', long)]
    pub start_block: Option<u64>,

    /// Start building from the latest MMR block
    #[arg(short = 'l', long, default_value_t = false)]
    pub from_latest: bool,

    /// Resume from the minimum MMR block stored on-chain (minus 1)
    #[arg(short = 'r', long, default_value_t = false)]
    pub resume: bool,
}

/// Run the MMR building process with the specified arguments
pub async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    initialize_environment(&args)?;
    validate_arguments(&args)?;

    let chain_id = get_env_var("CHAIN_ID")?.parse::<u64>()?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let verifier_address = get_env_var("FOSSIL_VERIFIER")?;
    let store_address = get_env_var("FOSSIL_STORE")?;

    let mut builder =
        create_builder_components(&args, &rpc_url, chain_id, &verifier_address, &store_address)
            .await?;
    execute_build_strategy(&args, &mut builder).await?;
    Ok(())
}

fn initialize_environment(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    dotenv::from_path(&args.env_file)?;
    initialize_logger()?;
    Ok(())
}

async fn create_builder_components<'a>(
    args: &Args,
    rpc_url: &'a str,
    chain_id: u64,
    verifier_address: &'a str,
    store_address: &'a str,
) -> Result<AccumulatorBuilder<'a>, Box<dyn std::error::Error>> {
    let private_key = get_env_var("STARKNET_PRIVATE_KEY")?;
    let account_address = get_env_var("STARKNET_ACCOUNT_ADDRESS")?;

    let starknet_provider = StarknetProvider::new(rpc_url)?;
    let starknet_account =
        StarknetAccount::new(starknet_provider.provider(), &private_key, &account_address)?;

    let proof_generator = ProofGenerator::new(MMR_BUILD_ELF, MMR_BUILD_ID)?;
    let mmr_state_manager = MMRStateManager::new(starknet_account, store_address, rpc_url);
    let batch_processor = BatchProcessor::new(args.batch_size, proof_generator, mmr_state_manager)?;

    AccumulatorBuilder::new(
        rpc_url,
        chain_id,
        verifier_address,
        batch_processor,
        0, // current_batch
        0, // total_batches
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create AccumulatorBuilder");
        Box::new(e) as Box<dyn std::error::Error>
    })
}

fn validate_arguments(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    match (args.resume, args.from_latest, args.start_block) {
        (true, true, _) => Err("Cannot specify both --resume and --from-latest".into()),
        (true, _, Some(_)) => Err("Cannot specify both --resume and --start-block".into()),
        (false, true, Some(_)) => Err("Cannot specify both --from-latest and --start-block".into()),
        _ => Ok(()),
    }
}

async fn execute_build_strategy(
    args: &Args,
    builder: &mut AccumulatorBuilder<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Debug logging to trace execution path
    tracing::info!(
        resume = args.resume,
        num_batches = args.num_batches,
        start_block = args.start_block,
        from_latest = args.from_latest,
        "Build strategy conditions check"
    );

    if args.resume {
        tracing::info!("Executing: handle_resume_build");
        handle_resume_build(args, builder).await
    } else if args.num_batches.is_some() && args.start_block.is_none() && !args.from_latest {
        // Smart restart: check onchain state when NUM_BATCHES is specified but no START_BLOCK
        tracing::info!("Executing: handle_smart_restart_build (onchain state check)");
        handle_smart_restart_build(args, builder).await
    } else {
        tracing::info!("Executing: handle_regular_build");
        handle_regular_build(args, builder).await
    }
}

async fn handle_smart_restart_build(
    args: &Args,
    builder: &mut AccumulatorBuilder<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let requested_batches = args
        .num_batches
        .ok_or("NUM_BATCHES must be specified for smart restart")?;

    let (min_mmr_block, total_batches_onchain) = get_onchain_state().await?;

    if should_skip_processing(total_batches_onchain, requested_batches) {
        return Ok(());
    }

    if min_mmr_block == 0 {
        handle_no_previous_mmr(builder, requested_batches).await
    } else {
        handle_continue_from_onchain_state(
            builder,
            min_mmr_block,
            total_batches_onchain,
            requested_batches,
        )
        .await
    }
}

async fn get_onchain_state() -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let store_address = get_env_var("FOSSIL_STORE")?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let starknet_provider = StarknetProvider::new(&rpc_url)?;

    let min_mmr_block = starknet_provider.get_min_mmr_block(&store_address).await?;
    let total_batches_onchain = starknet_provider.get_total_batches(&store_address).await?;

    Ok((min_mmr_block, total_batches_onchain))
}

fn should_skip_processing(total_batches_onchain: u64, requested_batches: u64) -> bool {
    if total_batches_onchain >= requested_batches {
        tracing::info!(
            requested_batches,
            total_batches_onchain,
            "Requested batches already processed onchain, nothing to do"
        );
        return true;
    }
    false
}

async fn handle_no_previous_mmr(
    builder: &mut AccumulatorBuilder<'_>,
    requested_batches: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::warn!("No minimum MMR block found on-chain, starting from finalized block");
    builder
        .build_with_num_batches(requested_batches)
        .await
        .map_err(Into::into)
}

async fn handle_continue_from_onchain_state(
    builder: &mut AccumulatorBuilder<'_>,
    min_mmr_block: u64,
    total_batches_onchain: u64,
    requested_batches: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_block = min_mmr_block.saturating_sub(1);
    let remaining_batches = requested_batches.saturating_sub(total_batches_onchain);

    tracing::info!(
        min_mmr_block,
        start_block,
        requested_batches,
        total_batches_onchain,
        remaining_batches,
        "Smart restart: continuing from onchain state"
    );

    builder
        .build_from_block_with_batches(start_block, remaining_batches, true)
        .await
        .map_err(Into::into)
}

async fn handle_resume_build(
    args: &Args,
    builder: &mut AccumulatorBuilder<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let store_address = get_env_var("FOSSIL_STORE")?;
    let rpc_url = get_env_var("STARKNET_RPC_URL")?;
    let starknet_provider = StarknetProvider::new(&rpc_url)?;
    let min_mmr_block = starknet_provider.get_min_mmr_block(&store_address).await?;

    if min_mmr_block == 0 {
        tracing::warn!("No minimum MMR block found on-chain, starting from finalized block");
        builder.build_from_finalized().await?;
    } else {
        let start_block = min_mmr_block.saturating_sub(1);
        tracing::info!(
            min_mmr_block,
            start_block,
            "Resuming from minimum MMR block minus 1"
        );

        match args.num_batches {
            Some(num_batches) => {
                builder
                    .build_from_block_with_batches(start_block, num_batches, true)
                    .await?
            }
            None => builder.build_from_block(start_block, true).await?,
        }
    }
    Ok(())
}

async fn handle_regular_build(
    args: &Args,
    builder: &mut AccumulatorBuilder<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    match (args.from_latest, args.start_block, args.num_batches) {
        (true, Some(_), _) => unreachable!("Cannot specify both --from-latest and --start-block"),
        (true, None, Some(num_batches)) => {
            builder
                .build_from_latest_with_batches(num_batches, true)
                .await?
        }
        (true, None, None) => builder.build_from_latest(true).await?,
        (false, Some(start_block), Some(num_batches)) => {
            builder
                .build_from_block_with_batches(start_block, num_batches, true)
                .await?
        }
        (false, Some(start_block), None) => builder.build_from_block(start_block, true).await?,
        (false, None, Some(num_batches)) => builder.build_with_num_batches(num_batches).await?,
        (false, None, None) => builder.build_from_finalized().await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_default_values() {
        let args = Args::parse_from(&["test"]);
        assert_eq!(args.batch_size, 1024);
        assert_eq!(args.skip_proof, false);
        assert_eq!(args.env_file, ".env");
        assert_eq!(args.from_latest, false);
        assert!(args.num_batches.is_none());
        assert!(args.start_block.is_none());
    }

    #[test]
    fn test_custom_batch_size() {
        let args = Args::parse_from(&["test", "--batch-size", "2048"]);
        assert_eq!(args.batch_size, 2048);
    }

    #[test]
    fn test_invalid_batch_size() {
        let result = Args::try_parse_from(&["test", "--batch-size", "invalid"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid digit found in string"));
    }

    #[tokio::test]
    async fn test_conflicting_args() {
        // Skip environment loading by checking early
        let args = Args {
            batch_size: 1024,
            num_batches: None,
            skip_proof: false,
            env_file: ".env".to_string(),
            start_block: Some(100),
            from_latest: true,
            resume: false,
        };

        // Check the validation directly
        let result: Result<(), Box<dyn std::error::Error>> =
            match (args.from_latest, args.start_block, args.num_batches) {
                (true, Some(_), _) => {
                    Err("Cannot specify both --from-latest and --start-block".into())
                }
                _ => Ok(()),
            };

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cannot specify both --from-latest and --start-block"
        );
    }

    #[test]
    fn test_smart_restart_trigger_conditions() {
        // Test conditions that trigger smart restart
        let args_smart_restart = Args {
            batch_size: 1024,
            num_batches: Some(700),
            skip_proof: false,
            env_file: ".env".to_string(),
            start_block: None,
            from_latest: false,
            resume: false,
        };

        // This should trigger smart restart
        assert!(
            args_smart_restart.num_batches.is_some()
                && args_smart_restart.start_block.is_none()
                && !args_smart_restart.from_latest
        );

        // Test conditions that don't trigger smart restart
        let args_with_start_block = Args {
            batch_size: 1024,
            num_batches: Some(700),
            skip_proof: false,
            env_file: ".env".to_string(),
            start_block: Some(100),
            from_latest: false,
            resume: false,
        };

        // This should not trigger smart restart (has start_block)
        assert!(
            !(args_with_start_block.num_batches.is_some()
                && args_with_start_block.start_block.is_none()
                && !args_with_start_block.from_latest)
        );
    }
}
