mod models;
mod capi;
mod chopper;
use chopper::run_the_chopper;
use clap::Parser;
use models::SummarisedContent;
use std::error::Error;
use reqwest::Client;
use capi::make_capi_request;

#[derive(Parser)]
pub struct Cli {
    capi_key:String,
    query_tag:String,
    output_path:String,
    limit:u16,
    page_size:u16,
    drop_no_summary:bool
}

pub async fn run(args:Cli) -> Result<(), Box<dyn Error>> {
    let http_client = Client::builder().build()?;

    let mut page_counter = 1;
    let mut summaries:Vec<SummarisedContent> = Vec::new();

    loop {
        let content = make_capi_request(&http_client, args.capi_key.to_owned(), args.query_tag.to_owned(), page_counter, u32::from(args.page_size)).await?;

        let summaries_page = content.response.results.iter()
            .map(|doc| run_the_chopper(&doc.blocks))
            .flatten();
        summaries.extend(summaries_page);

    }
}