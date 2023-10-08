use clap::Parser;
use liveblog_data_xtractor_rust::{Cli, run};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let args = Cli::parse();

    run(args).await
}
