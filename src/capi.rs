use std::time::Duration;
use std::error::Error;
use crate::models::*;
use reqwest::StatusCode;
use serde_json::from_slice;
use std::fmt::Display;
use std::collections::HashMap;
use itertools::Itertools;
use std::future::Future;
use std::any::Any;

#[derive(Debug)]
pub struct CapiError {
    code:u16,
    msg:String
}

impl Display for CapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("CAPI error {}: {}", self.code, self.msg))
    }
}

impl Error for CapiError {

}

impl CapiError {
    pub fn new(code:StatusCode, msg:&str) -> CapiError {
        CapiError { code: code.as_u16(), msg: msg.to_owned() }
    }

    pub fn should_retry(&self) -> bool {
        self.code==503 || self.code==504
    }
}

async fn internal_make_request(client: &reqwest::Client, url:&str) -> Result<CapiResponseEnvelope, Box<dyn Error>> {
    let response = client.get(url).send().await?;
    let status = response.status();
    let body = response.bytes().await?;

    if status==200 {
        match serde_json::from_slice(&body) {
            Ok(content)=>return Ok(content),
            Err(e)=>{
                println!("ERROR could not unmarshal content: {}", e);
                let body_string:Vec<u8> = body.into_iter().collect();
                let content_string = String::from_utf8(body_string).unwrap_or(String::from("(not utf)"));
                println!("Body was: {}", content_string);
                return Err(Box::new(e))
            }
        }
    } else {
        let content = std::str::from_utf8(&body).unwrap_or("invalid UTF data");
        return Err(Box::new(CapiError::new(status, content)));
    }
}

/// Public method to request content from the Content Application Programmer's Interface.
/// # Arguments:
/// 
/// * `client` - Immutable reference to an HTTP client (provided by Reqwest) for making the http requests with
/// * `capi_key` - String of the API key to use
/// * `query_tag` - Tags query to use. This takes the form of a comma-separated list of tag IDs (for AND) or a pipe-separated list of tag IDs (for OR). Any tag ID can be negated by appending a - sign
/// * `page_counter` - Number of the page to retrieve. Pages start at 1.
/// * `page_size` - Number of items to retrieve on a page
/// * `retry_delay` - a Duration representing the amount of time to wait between unsuccessful requests. Note that there is no retry for 4xx requests.
pub async fn make_capi_request(client: &reqwest::Client, capi_key:String, query_tag:String, page_counter:u64, page_size:u32, retry_delay:Option<Duration>) -> Result<CapiResponseEnvelope, Box<dyn Error>> {
    let args = HashMap::from([
        ("api-key", capi_key),
        ("show-tags", String::from("all")),
        ("tag", query_tag),
        ("show-blocks", String::from("all")),
        ("page", format!("{}", page_counter)),
        ("page-size", format!("{}", page_size))
    ]);

    let argstring:String = args.iter()
        .map(|(k,v)| format!("{}={}", k, url_escape::encode_fragment(v)))
        .intersperse(String::from("&"))
        .collect();
    
    let url = format!("https://content.guardianapis.com/search?{}", argstring);

    loop {
        match internal_make_request(client, &url).await {
            Ok(content)=>return Ok(content),
            Err(err)=>
                match err.downcast_ref::<CapiError>() {
                    Some(capi_err)=>
                        if capi_err.should_retry() {
                            std::thread::sleep(retry_delay.unwrap_or(Duration::from_secs(2)));
                            continue;
                        } else {
                            return Err(err);
                        },
                    None=>return Err(err)
                }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn make_capi_request_success() {

    }
}