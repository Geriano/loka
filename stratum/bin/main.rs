use anyhow::Result;
use loka_stratum::cli;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    cli::run().await
}
