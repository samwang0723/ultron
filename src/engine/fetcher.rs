use anyhow::{anyhow, Result};
use async_trait::async_trait;
use encoding_rs::*;
use once_cell::sync::Lazy;
use reqwest::StatusCode;
use tokio::fs;

#[cfg(feature = "testing")]
static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .build()
        .expect("Failed to create Client")
});

// Define a static instance of `Client` which will be initialized on the first use
#[cfg(not(feature = "testing"))]
static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let account = std::env::var("PROXY_USER").unwrap();
    let password = std::env::var("PROXY_PASSWD").unwrap();
    let proxy_url = format!("http://{}:{}@gate.smartproxy.com:7000", account, password);
    let proxy = reqwest::Proxy::https(proxy_url).expect("Failed to create proxy");
    reqwest::Client::builder()
        .proxy(proxy)
        // Optionally configure the client
        .build()
        .expect("Failed to create Client")
});

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

struct UrlFetcher<'a>(pub(crate) &'a str);
struct FileFetcher<'a>(pub(crate) &'a str);

#[async_trait]
impl<'a> Fetch for UrlFetcher<'a> {
    type Error = anyhow::Error;

    async fn fetch(&self) -> Result<Payload, Self::Error> {
        let resp = CLIENT.get(self.0).send().await?;
        match resp.status() {
            StatusCode::OK => {
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .map(|v| v.to_str().unwrap_or_default().to_owned())
                    .unwrap_or_default();

                // charset checking
                let mut charset = "utf-8";
                for (k, v) in resp.headers().iter() {
                    // check if contains charset=ms950
                    let value = v.to_str().unwrap_or_default();
                    if k == "content-type" && (value.contains("ms950") || value.contains("csv")) {
                        charset = "big5";
                        break;
                    }
                }
                let raw_body = resp.bytes().await?;
                let body = match charset {
                    "big5" => self.decode_big5(&raw_body)?,
                    _ => String::from_utf8(raw_body.to_vec()).unwrap(),
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
