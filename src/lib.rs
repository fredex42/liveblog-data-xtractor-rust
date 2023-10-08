mod models;
mod capi;
mod chopper;
mod writer;
use chopper::run_the_chopper;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use writer::write_out_data;
use clap::Parser;
use models::{SummarisedContent, Stats, CapiTag};
use std::{error::Error, time::SystemTime};
use std::iter::Filter;
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

fn filter_tags_by_type<'a>(tags:&'a [CapiTag], tag_type:&'a str) -> impl Iterator<Item = &'a CapiTag> {
    tags.iter().filter(move |t| t.r#type==tag_type)
}

pub async fn run(args:Cli) -> Result<(), Box<dyn Error>> {
    let http_client = Client::builder().build()?;

    let mut page_counter = 1;
    let mut summaries:Vec<SummarisedContent> = Vec::new();

    loop {
        let content = make_capi_request(&http_client, args.capi_key.to_owned(), args.query_tag.to_owned(), page_counter, u32::from(args.page_size)).await?;

        if content.response.results.len()==0 {
            println!("INFO Reached the last page of results, finishing");
            return Ok(());
        }
        
        for liveblog in content.response.results.iter() {
            let summaries = run_the_chopper(&liveblog.blocks);

            let now:DateTime<Utc> = SystemTime::now().clone().into();
            
            let stats = Stats {
                original_id: &liveblog.id,
                web_publication_date: liveblog.webPublicationDate,
                retrieved_at: now.clone().into(),
                summary_block_count: liveblog.blocks.count_summary_blocks(),
                total_block_count: liveblog.blocks.count_body_blocks(),
                keyword_tags: filter_tags_by_type(&liveblog.tags, "keyword").map(|t| t.clone()).collect_vec(),
            };

            match write_out_data(&args.output_path, &liveblog.id, &summaries, &stats) {
                Ok(_) => (),
                Err(e)=> {
                    return Err(e);
                }
            }
        }

        page_counter+=1;
    }

}