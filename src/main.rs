use std::process::ExitCode;

use env_logger::Env;
use log::{error, info};

use crate::parser::Parser;

mod parser;
mod terraform;

async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let parser = parser::DirectoryParser::new();
    let terraform = parser.parse("terraform").await?;

    info!("{terraform:?}");

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = _main().await {
        error!("{e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
