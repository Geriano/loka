use anyhow::Result;
use loka_stratum::cli;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}
