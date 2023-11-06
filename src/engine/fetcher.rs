use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::StatusCode;
use tokio::fs;

#[derive(Debug)]
pub struct Payload {
    pub content: String,
    pub source: String,
    pub content_type: String,
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
        let resp = reqwest::get(self.0).await?;
        match resp.status() {
            StatusCode::OK => {
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .map(|v| v.to_str().unwrap_or_default().to_owned())
                    .unwrap_or_default();

                let body = resp.text().await?;
                Ok(Payload {
                    content: body,
                    source: self.0.to_owned(),
                    content_type,
                })
            }
            StatusCode::NOT_FOUND => Err(anyhow!("Not found")),
            _ => Err(anyhow!("Failed to fetch url: {}", self.0)),
        }
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
