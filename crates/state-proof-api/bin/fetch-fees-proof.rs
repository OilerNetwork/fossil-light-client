use clap::Parser;
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
    let args = Args::parse();
    
    let client = Client::new();
    let url = format!(
        "{}/verify-blocks?from_block={}&to_block={}", 
        args.api_url,
        args.from_block,
        args.to_block
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
        eprintln!("API request failed: {}", error_text);
        std::process::exit(1);
    }

    let bytes = response.bytes().await?;
    let stark: Stark = bincode::deserialize(&bytes)?;

    println!("Successfully retrieved and deserialized proof:");
    println!("Image ID length: {}", stark.image_id().unwrap().len());
    println!("Receipt: {:?}", stark.receipt());

    Ok(())
}