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
        let content:CapiResponseEnvelope = serde_json::from_slice(&body)?;
        return Ok(content);
    } else {
        let content = std::str::from_utf8(&body).unwrap_or("invalid UTF data");
        return Err(Box::new(CapiError::new(status, content)));
    }
}

async fn with_retry<T, F>(client: &reqwest::Client, mut f:F, retry_delay:Duration) -> Result<T, Box<dyn Error>>
where
    T: Any,
    F: FnMut() -> Result<T, Box<dyn Error>>,
    //F: FnMut() -> dyn Future<Output = Result<T, Box<dyn Error>>,
    //F: FnMut() -> dyn Future<Output = Result<T, Box<dyn Error>>,
{
    loop {
        match f() {
            Ok(content)=>return Ok(content),
            Err(err)=>
                match err.downcast_ref::<CapiError>() {
                    Some(capi_err)=>
                        if capi_err.should_retry() {
                            std::thread::sleep(retry_delay);
                            continue;
                        } else {
                            return Err(err);
                        }
                    None=>
                        return Err(err)
                }
        }
    }
}

pub async fn make_capi_request(client: &reqwest::Client, capi_key:String, query_tag:String, pageCounter:u64, pageSize:u32) -> Result<CapiResponseEnvelope, Box<dyn Error>> {
    let args = HashMap::from([
        ("api-key", capi_key),
        ("show-tags", String::from("all")),
        ("tag", query_tag),
        ("show-blocks", String::from("all")),
        ("page", format!("{}", pageCounter)),
        ("page-size", format!("{}", pageSize))
    ]);

    let argstring:String = args.iter()
        .map(|(k,v)| format!("{}={}", k, url_escape::encode_fragment(v)))
        .intersperse(String::from("&"))
        .collect();
    
    let url = format!("https://content.guardianapis.com/search?{}", argstring);

    return internal_make_request(client, &url).await;

    //return with_retry(client, || internal_make_request(client, &url).await?, Duration::from_secs(5))

    // match internal_make_request(client, &url).await {
    //     Ok(content)=> Ok(content),
    //     Err(err)=>
    //         match err.downcast_ref::<CapiError>() {
    //             Some(capi_err)=>
    //                 if capi_err.should_retry() {
    //                     std::thread::sleep(Duration::from_secs(5));
    //                     return make_capi_request(client, capi_key, query_tag, pageCounter, pageSize).await
    //                 } else {
    //                     return Err(err)
    //                 }
    //             None=>Err(err)
    //         }
    // }
}