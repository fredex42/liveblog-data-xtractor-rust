use crate::models::*;
use std::error::Error;
use core::slice::Iter;

fn recursive_chopper(mut i:Iter<CapiBlock>, mut summaries:Vec<SummarisedContent>, mut non_summary_blocks:Vec<CapiBlock>) -> Vec<SummarisedContent>{
    match i.next() {
        Some(block)=>
            if block.attributes.summary {   //we reached a summary, start a new block of summarised content
                summaries.push(SummarisedContent::new(block.clone(), non_summary_blocks));
                return recursive_chopper(i, summaries, Vec::new());
            } else {
                non_summary_blocks.push(block.clone());
                return recursive_chopper(i, summaries, non_summary_blocks)
            }
        None => {
            summaries.push(SummarisedContent::unsummarised(non_summary_blocks));
            return summaries;
        }
    }
}

pub fn run_the_chopper(blocks:&CapiBlocksContainer) -> Vec<SummarisedContent> {
    let summarised_content = recursive_chopper(blocks.body.iter(), Vec::new(), Vec::new());

    return summarised_content;
}

#[cfg(test)]
mod test {
    
}