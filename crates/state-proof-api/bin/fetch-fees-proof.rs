use clap::Parser;
use common::initialize_logger_and_env;
use methods::VALIDATE_BLOCKS_AND_EXTRACT_FEES_ID;
use publisher::utils::Stark;
use reqwest::Client;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Starting block number
    #[arg(long)]
    from_block: u64,

    /// Ending block number
    #[arg(long)]
    to_block: u64,

    /// Skip proof verification
    #[arg(long)]
    skip_proof_verification: Option<bool>,

    /// API endpoint URL
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    api_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logger_and_env()?;
    let args = Args::parse();

    let client = Client::new();
    let url = format!(
        "{}/verify-blocks?from_block={}&to_block={}",
        args.api_url, args.from_block, args.to_block
    );

    // Add skip_proof_verification to URL if provided
    let url = if let Some(skip) = args.skip_proof_verification {
        format!("{}&skip_proof_verification={}", url, skip)
    } else {
        url
    };

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        tracing::error!("API request failed: {}", error_text);
        std::process::exit(1);
    }

    let bytes = response.bytes().await?;
    tracing::info!("Received {} bytes from API", bytes.len());

    match bincode::deserialize::<Vec<Stark>>(&bytes) {
        Ok(stark_vec) => {
            tracing::info!(
                "Successfully retrieved and deserialized {} proofs:",
                stark_vec.len()
            );
            let mut all_fees: Vec<u64> = Vec::new();
            for (_i, stark) in stark_vec.iter().enumerate() {
                let decoded_journal = stark.receipt().journal.decode::<Vec<String>>()?;
                let fees: Vec<u64> = decoded_journal
                    .iter()
                    .map(|hex| u64::from_str_radix(&hex[2..], 16))
                    .collect::<Result<_, _>>()?;
                all_fees.extend(fees.iter());
                tracing::info!("Decoded fees: {:?}", fees);
                stark
                    .receipt()
                    .verify(VALIDATE_BLOCKS_AND_EXTRACT_FEES_ID)?;
                tracing::info!(
                    "Stark proof for block fees in range {} to {} verified successfully",
                    args.from_block,
                    args.to_block
                );
            }
            tracing::info!("All consolidated fees: {:?}", all_fees);
        }
        Err(e) => {
            tracing::error!("Failed to deserialize response: {}", e);
            tracing::error!(
                "First 100 bytes of response: {:?}",
                &bytes.get(..100.min(bytes.len()))
            );
            tracing::error!("Detailed error: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
