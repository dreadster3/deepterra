use std::process::ExitCode;

use crate::parser::Parser;

mod parser;
mod terraform;

async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = parser::DirectoryParser::new();
    let terraform = parser.parse("terraform").await?;

    println!("{terraform:?}");

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = _main().await {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
