mod aws_service;
mod cli;
mod dynamodb;
mod error;
use crate::cli::Args;
use crate::error::CustomError;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), CustomError> {
    let args = Args::parse();

    args.aws_service.execute(args.profile).await?;

    Ok(())
}
