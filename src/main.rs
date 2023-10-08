use clap::Parser;
use liveblog_data_xtractor_rust::{Cli, run};
use std::error::Error;

//we need to wrap in the Tokio macro in order to set up a thread pool for async operations
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let args = Cli::parse();

    run(args).await
}
