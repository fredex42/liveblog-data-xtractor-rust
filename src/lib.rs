mod models;
mod capi;
mod chopper;
mod writer;
use chopper::run_the_chopper;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use writer::write_out_data;
use clap::Parser;
use models::{Stats, CapiTag};
use std::{error::Error, time::SystemTime, path::PathBuf};
use reqwest::Client;
use capi::make_capi_request;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short,long)]
    capi_key:String,
    #[arg(short,long)]
    query_tag:String,
    #[arg(short,long)]
    output_path:Option<String>,
    #[arg(short,long)]
    limit:u16,
    #[arg(short,long)]
    page_size:Option<u32>,
    #[arg(short,long)]
    drop_no_summary:bool
}

fn filter_tags_by_type<'a>(tags:&'a [CapiTag], tag_type:&'a str) -> impl Iterator<Item = &'a CapiTag> {
    tags.iter().filter(move |t| t.r#type==tag_type)
}

pub async fn run(args:Cli) -> Result<(), Box<dyn Error>> {
    let http_client = Client::builder().build()?;

    let mut page_counter = 1;

    let output_path = args.output_path.unwrap_or_else(|| {
        match std::env::current_dir() {
            Ok(p)=> {
                let s = p.as_path().as_os_str().to_str().unwrap_or("/");
                String::from(s)
            },
            Err(e)=>{
                println!("ERROR Could not get current working directory: {}", e);
                String::from("/")
            }
        }
    });

    loop {
        let content = make_capi_request(&http_client, 
            args.capi_key.to_owned(), 
            args.query_tag.to_owned(), 
            page_counter, 
            u32::from(args.page_size.unwrap_or(10))).await?;

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

            match write_out_data(&output_path, &liveblog.id, &summaries, &stats) {
                Ok(_) => (),
                Err(e)=> {
                    return Err(e);
                }
            }
        }

        page_counter+=1;
    }

}