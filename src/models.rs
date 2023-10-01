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

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiBlock {
    pub id:String,
    pub bodyHtml:String,
    pub attributes:CapiBlockAttributes,
    pub firstPublishedDate:String,
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
    id:String,
    r#type: String,
    webPublicationDate: DateTime<FixedOffset>,
    blocks: CapiBlocksContainer,
    tags: Vec<CapiTag>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiResponse {
    status:String,
    userTier:String,
    total: u64,
    startIndex: u64,
    pageSize: u32,
    currentPage: u64,
    pages: u64,
    orderBy: String,
    results: Vec<CapiDocument>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CapiResponseEnvelope {
    response:CapiResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SummarisedContent {
    summary:CapiBlock,
    events: Vec<CapiBlock>,
}

impl SummarisedContent {
    pub fn is_empty_summary(&self) -> bool {
        return self.summary.bodyHtml == "" && self.summary.id == ""
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Stats {
    original_id:String,
    web_publication_date: DateTime<FixedOffset>,
    retrieved_at: DateTime<FixedOffset>,
    summary_block_count: u64,
    total_block_count: u64,
    keyword_tags: Vec<CapiTag>,
}

impl Stats {
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
            original_id: "original-id-here".to_owned(),
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
