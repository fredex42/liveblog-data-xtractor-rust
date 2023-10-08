use serde::{Deserialize, Serialize};
use chrono::{DateTime, TimeZone, FixedOffset};
use std::io;
use std::str;

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiBlockAttributes {
    pub summary:bool,
    pub title:Option<String>,
    pub pinned:bool,
}

impl CapiBlockAttributes {
    pub fn clone(&self) -> CapiBlockAttributes {
        CapiBlockAttributes { 
            summary: self.summary, 
            title: match &self.title {
                Some(t)=>Some(t.to_owned()),
                None=>None
            },
            pinned: self.pinned
         }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiBlock {
    pub id:String,
    pub bodyHtml:String,
    pub attributes:CapiBlockAttributes,
    pub firstPublishedDate:String,
}

impl CapiBlock {
    pub fn clone(&self) -> CapiBlock {
        CapiBlock { 
            id: self.id.to_owned(), 
            bodyHtml: self.bodyHtml.to_owned(), 
            attributes: self.attributes.clone(),
            firstPublishedDate: self.firstPublishedDate.to_owned(),
         }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiBlocksContainer {
    pub main:CapiBlock,
    pub body:Vec<CapiBlock>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiTag {
    pub id:String,
    pub webTitle:String,
    pub r#type:String,
}

impl Clone for CapiTag {
    fn clone(&self)->Self {
        CapiTag { id: self.id.to_owned(), webTitle: self.webTitle.to_owned(), r#type: self.r#type.to_owned() }
    }
}

impl CapiBlocksContainer {
    pub fn count_body_blocks(&self) -> usize {
        return self.body.len();
    }

    pub fn count_summary_blocks(&self) -> usize {
        self.body.iter().filter(|b| b.attributes.summary).count()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiDocument {
    pub id:String,
    pub r#type: String,
    pub webPublicationDate: DateTime<FixedOffset>,
    pub blocks: CapiBlocksContainer,
    pub tags: Vec<CapiTag>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiResponse {
    pub status:String,
    pub userTier:String,
    pub total: u64,
    pub startIndex: u64,
    pub pageSize: u32,
    pub currentPage: u64,
    pub pages: u64,
    pub orderBy: String,
    pub results: Vec<CapiDocument>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiResponseEnvelope {
    pub response:CapiResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SummarisedContent {
    pub summary: Option<CapiBlock>,
    pub events: Vec<CapiBlock>,
}

impl SummarisedContent {
    pub fn empty() -> SummarisedContent {
        SummarisedContent { summary: None, events: vec!() }
    }

    pub fn new(summary:CapiBlock, events: Vec<CapiBlock>) -> SummarisedContent {
        SummarisedContent {
            summary: Some(summary),
            events: events,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Stats<'a> {
    pub original_id:&'a str,
    pub web_publication_date: DateTime<FixedOffset>,
    pub retrieved_at: DateTime<FixedOffset>,
    pub summary_block_count: usize,
    pub total_block_count: usize,
    pub keyword_tags: Vec<CapiTag>,
}

impl Stats<'_> {
    #[inline]
    pub fn write_json(&self, to: &mut dyn io::Write) -> Result<(), serde_json::Error> {
        return serde_json::to_writer(to, self)
    }

    pub fn write_json_buffer(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut buf:Vec<u8> = vec!();
        self.write_json(&mut buf)?;
        return Ok(buf);
    }

    pub fn write_json_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        let buf = self.write_json_buffer()?;
        let str = String::from_utf8(buf)?;
        return Ok(str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_count_empty_capi_blocks() {
        let to_test: CapiBlocksContainer = CapiBlocksContainer {
            main: CapiBlock {
                id: "fred".to_owned(),
                bodyHtml: "<b>Test</b".to_owned(),
                attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
            },
            body: vec!(),
        };

        assert_eq!(to_test.count_body_blocks(), 0);
    }

    #[test]
    pub fn test_count_multiple_capi_blocks() {
        let to_test: CapiBlocksContainer = CapiBlocksContainer {
            main: CapiBlock {
                id: "fred".to_owned(),
                bodyHtml: "<b>Test</b".to_owned(),
                attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
            },
            body: vec!(
                CapiBlock {
                    id: "fred".to_owned(),
                    bodyHtml: "<b>Test</b".to_owned(),
                    attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                    firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
                },
                CapiBlock {
                    id: "kate".to_owned(),
                    bodyHtml: "<b>Test</b".to_owned(),
                    attributes: CapiBlockAttributes { summary: true, title: Some("this is a summary".to_owned()), pinned: false },
                    firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
                },
                CapiBlock {
                    id: "bob".to_owned(),
                    bodyHtml: "<b>Test</b".to_owned(),
                    attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                    firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
                },
            ),
        };

        assert_eq!(to_test.count_body_blocks(), 3);
    }

    #[test]
    pub fn test_count_summary_blocks() {
        let to_test: CapiBlocksContainer = CapiBlocksContainer {
            main: CapiBlock {
                id: "fred".to_owned(),
                bodyHtml: "<b>Test</b".to_owned(),
                attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
            },
            body: vec!(
                CapiBlock {
                    id: "fred".to_owned(),
                    bodyHtml: "<b>Test</b".to_owned(),
                    attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                    firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
                },
                CapiBlock {
                    id: "kate".to_owned(),
                    bodyHtml: "<b>Test</b".to_owned(),
                    attributes: CapiBlockAttributes { summary: true, title: Some("this is a summary".to_owned()), pinned: false },
                    firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
                },
                CapiBlock {
                    id: "bob".to_owned(),
                    bodyHtml: "<b>Test</b".to_owned(),
                    attributes: CapiBlockAttributes { summary: false, title: None, pinned: false },
                    firstPublishedDate: "2022-01-02T03:04:05Z".to_owned()
                },
            ),
        };

        assert_eq!(to_test.count_summary_blocks(), 1);
    }

    #[test]
    pub fn test_write_stats_json() {
        let to_test = Stats {
            original_id: "original-id-here",
            web_publication_date: DateTime::parse_from_rfc3339("2022-01-02T03:04:05.678Z").unwrap(),
            retrieved_at: DateTime::parse_from_rfc3339("2022-01-02T03:04:05.678Z").unwrap(),
            summary_block_count: 1,
            total_block_count: 5,
            keyword_tags: vec!(),
        };

        let expected = "{\"original_id\":\"original-id-here\",\"web_publication_date\":\"2022-01-02T03:04:05.678Z\",\"retrieved_at\":\"2022-01-02T03:04:05.678Z\",\"summary_block_count\":1,\"total_block_count\":5,\"keyword_tags\":[]}";
        let marshalled = to_test.write_json_string().unwrap();
        assert_eq!(marshalled, expected);
    }
}
