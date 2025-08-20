pub mod args;
pub mod commands;

pub use args::{Args, Commands};

use anyhow::Result;

pub async fn run() -> Result<()> {
    let args = Args::parse();
    commands::execute(args).await
}