use std::str;
use std::error::Error;
use std::fs::{create_dir_all, File};
use crate::{models::*, capi};

fn dir_name_from_capi_id(capi_id:&str) -> &str {
    let id_parts = str::split(capi_id, "/");
    match id_parts.last() {
        Some(dirname)=>
        if dirname.len()>200 {
            let (head, _) = dirname.split_at(200);
            return head;
        } else {
            return dirname;
        }
        None=>"UNKNOWN"
    }
}

fn write_block_to_file(file_name:&String, b:&SummarisedContent) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file_name)?;
    match serde_json::to_writer(file, b) {
        Ok(_)=>Ok(()),
        Err(e)=>Err(Box::new(e))
    }
}

pub fn write_out_data(base_path:&str, capi_id:&str, chopped_blocks:&Vec<SummarisedContent>, stats:&Stats) -> Result<(), Box<dyn Error>> {
    let dir_name = dir_name_from_capi_id(capi_id);

    //deliberately ignore the error here, because the dir might already exist.
    let _ = create_dir_all(dir_name);

    //now write out all the summarised blocks we found
    for block in chopped_blocks.iter() {
        let id_to_use:String = block.summary.as_ref().map_or_else(|| "HEAD".to_owned(), |summ| summ.id.clone());
        let file_name = format!("{}/{}/{}.json", base_path, dir_name, id_to_use);
        write_block_to_file(&file_name, block)?
    }

    //finally write out the metadata stats
    Ok(())
}