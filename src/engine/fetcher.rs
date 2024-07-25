use anyhow::{anyhow, Result};
use async_trait::async_trait;
use encoding_rs::*;
use lazy_static::lazy_static;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use std::time::Duration;
use tokio::fs;

use crate::config::setting::SETTINGS;

#[cfg(feature = "testing")]
lazy_static! {
    static ref CLIENT: reqwest::Client = {
        reqwest::Client::builder()
            .build()
            .expect("Failed to create Client")
    };
}

// Define a static instance of `Client` which will be initialized on the first use
#[cfg(not(feature = "testing"))]
lazy_static! {
    static ref CLIENT: reqwest::Client =  {
        let proxy =
            reqwest::Proxy::https(SETTINGS.proxy.connection_string()).expect("Failed to create proxy");
        reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(60))
            // Optionally configure the client
            .build()
            .expect("Failed to create Client")
    };

    static ref NO_PROXY_CLIENT: reqwest::Client = {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create No Proxy Client")
    };
}

#[derive(Debug, Clone)]
pub struct Payload {
    pub content: String,
    pub source: String,
    pub content_type: String,
    pub date: Option<String>,
}

#[async_trait]
pub trait Fetch {
    type Error;
    async fn fetch(&self) -> Result<Payload, Self::Error>;
}

/// Exctract content from data source
pub async fn fetch_content(source: impl AsRef<str>) -> Result<Payload> {
    let name = source.as_ref();
    match &name[..4] {
        // including http / https
        "http" => UrlFetcher(name).fetch().await,
        // handle file://<filename>
        "file" => FileFetcher(name).fetch().await,
        _ => Err(anyhow!("Only support http/https/file at the moment")),
    }
}

fn crawling_client(url: &str) -> &'static reqwest::Client {
    if url.contains("www.twse.com.tw") || url.contains("www.tpex.org.tw") {
        &NO_PROXY_CLIENT
    } else {
        &CLIENT
    }
}

struct UrlFetcher<'a>(pub(crate) &'a str);
struct FileFetcher<'a>(pub(crate) &'a str);

#[async_trait]
impl<'a> Fetch for UrlFetcher<'a> {
    type Error = anyhow::Error;

    async fn fetch(&self) -> Result<Payload, Self::Error> {
        let target = self.0;
        let resp = crawling_client(target).get(target).send().await?;
        match resp.status() {
            StatusCode::OK => {
                let content_type = self.get_content_type(resp.headers());

                // Charset checking
                let charset = if ["ms950", "big5", "csv"]
                    .iter()
                    .any(|&s| content_type.contains(s))
                {
                    "big5"
                } else {
                    "utf-8"
                };

                let raw_body = resp.bytes().await?;
                let body = if charset == "big5" {
                    self.decode_big5(&raw_body)?
                } else {
                    // Safely handle potential UTF-8 conversion errors
                    String::from_utf8(raw_body.to_vec())
                        .map_err(|e| anyhow!("Failed to decode UTF-8: {}", e))?
                };

                Ok(Payload {
                    content: body,
                    source: self.0.to_owned(),
                    content_type,
                    date: None,
                })
            }
            StatusCode::NOT_FOUND => Err(anyhow!("Not found")),
            _ => Err(anyhow!("Failed to fetch url: {}", self.0)),
        }
    }
}

impl UrlFetcher<'_> {
    fn get_content_type(&self, headers: &HeaderMap) -> String {
        match headers.get("content-type") {
            Some(header_value) => match header_value.to_str() {
                Ok(value) => value.to_owned(),
                Err(_) => String::new(), // Handle the error case, e.g., log the error
            },
            None => String::new(), // Handle the case where the header is not present
        }
    }

    fn decode_big5(&self, input: &[u8]) -> Result<String, anyhow::Error> {
        let (decoded_content, _, had_errors) = BIG5.decode(input);
        if had_errors {
            // Handle decoding errors, maybe return a custom error
            return Err(anyhow!("Decoding error occurred"));
        }

        let utf8_string = decoded_content.into_owned();
        Ok(utf8_string)
    }
}

#[async_trait]
impl<'a> Fetch for FileFetcher<'a> {
    type Error = anyhow::Error;

    async fn fetch(&self) -> Result<Payload, Self::Error> {
        let body = fs::read_to_string(&self.0[7..]).await?;
        Ok(Payload {
            content: body,
            source: self.0.to_owned(),
            content_type: "text/plain".to_owned(),
            date: None,
        })
    }
}

// Testcases for fetch_content, using mock on http and files
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_content_http() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Use one of these addresses to configure your client
        let url = server.url();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("Hello World")
            .create_async()
            .await;

        let payload = fetch_content(url.as_str()).await.unwrap();
        assert_eq!(payload.content, "Hello World");
        assert_eq!(payload.source, url);
        assert_eq!(payload.content_type, "text/html");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_content_file() {
        let payload = fetch_content("file://Cargo.toml").await.unwrap();
        assert!(payload.content.contains("version"));
        assert_eq!(payload.source, "file://Cargo.toml");
        assert_eq!(payload.content_type, "text/plain");
    }
}
