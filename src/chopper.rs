use crate::models::*;
use core::slice::Iter;

fn recursive_chopper(mut i:Iter<CapiBlock>, mut summaries:Vec<SummarisedContent>, mut current:SummarisedContent) -> Vec<SummarisedContent>{
    match i.next() {
        Some(block)=>
            if block.attributes.summary.unwrap_or(false) {   //we reached a summary, start a new block of summarised content
                summaries.push(current);
                return recursive_chopper(i, summaries, SummarisedContent::new(block.clone(), vec!()));
            } else {
                current.events.push(block.clone());
                return recursive_chopper(i, summaries, current)
            }
        None => {
            summaries.push(current);
            return summaries;
        }
    }
}

pub fn run_the_chopper(blocks:&CapiBlocksContainer) -> Vec<SummarisedContent> {
    let summarised_content = recursive_chopper(blocks.body.iter(), Vec::new(), SummarisedContent::empty());

    return summarised_content;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dyn_fmt::AsStrFormatExt;

    fn gen_blocks(block_count:u32,template_text:&str, summary_at:&[u32]) -> Vec<CapiBlock> {
        let mut out:Vec<CapiBlock> = vec!();
        let mut summary_at_index:usize = 0;
        let mut i=block_count-1;

        while i>0 {
            let most_recent_summary:bool;

            if summary_at[summary_at_index]==i {
                most_recent_summary = true;
                if summary_at_index < summary_at.len()-1 {
                    summary_at_index += 1
                }
            } else {
                most_recent_summary = false;
            }

            out.push(CapiBlock { 
                id: format!("{}", i),
                bodyHtml: template_text.format(&[i]),
                attributes: CapiBlockAttributes { 
                    summary: Some(most_recent_summary),
                    title: Some(format!("Block {}", i)),
                    pinned: Some(false),
                },
                firstPublishedDate: None,
            });

            i-=1;
        }

        return out;
    }

    #[test]

    pub fn test_chopper_multi_summary() {
        let summary_locations = [90, 80, 65, 33, 4];
        let blocks= CapiBlocksContainer { 
            main: CapiBlock { 
                id: "fake-main".to_owned(),
                bodyHtml: "".to_owned(),
                attributes: CapiBlockAttributes { summary: Some(false), title: None, pinned: Some(false) },
                firstPublishedDate: None,
            },
            body: gen_blocks(99, "This is block number {}", &summary_locations),
        };
        let result = run_the_chopper(&blocks);

        assert_eq!(result.len(), 6);
        assert!(result[0].summary.is_none());
        assert_eq!(result[1].summary.as_ref().map(|v| v.id.as_str()), Some("90"));
        assert_eq!(result[1].events.len(), 9);
        assert_eq!(result[2].summary.as_ref().map(|v| v.id.as_str()), Some("80"));
        assert_eq!(result[2].events.len(), 14);
        assert_eq!(result[3].summary.as_ref().map(|v| v.id.as_str()), Some("65"));
        assert_eq!(result[3].events.len(), 31);
        assert_eq!(result[4].summary.as_ref().map(|v| v.id.as_str()), Some("33"));
        assert_eq!(result[4].events.len(), 28);
        assert_eq!(result[5].summary.as_ref().map(|v| v.id.as_str()), Some("4"));
        assert_eq!(result[5].events.len(), 3);
    }
}