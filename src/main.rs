mod engine;
use crate::engine::fetcher::{fetch_content, Payload};
use crate::engine::model::Model;
use crate::engine::parser::{ConcentrationStrategy, Parser};

use chrono::{Datelike, Local};
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

static CONCENTRATION_PAGES: usize = 5;
static PROXY_URL: &str = "https://api.webscrapingapi.com/v1";

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init();

    // List of URLs to process
    let stocks = vec!["2330", "2363", "8150"];

    let capacity = stocks.len() * CONCENTRATION_PAGES;
    let (url_tx, url_rx) = mpsc::channel(capacity);

    // retrieve all handles and ensure process not termiated before tasks completed
    let url_gen_handle = tokio::spawn(generate_urls(url_tx, stocks.clone()));
    let fetch_aggregate_handle = tokio::spawn(fetch_urls(url_rx, capacity));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(url_gen_handle, fetch_aggregate_handle);
}

async fn generate_urls(url_tx: mpsc::Sender<String>, stocks: Vec<&str>) {
    let proxy_api_key =
        env::var("PROXY_API_KEY").expect("PROXY_API_KEY not found in the environment");
    let proxy_url = format!("{}?api_key={}", PROXY_URL, proxy_api_key);
    for stock in stocks.iter() {
        for i in 1..=CONCENTRATION_PAGES {
            // skip the 40 days calculation
            let index = if i == CONCENTRATION_PAGES { i + 1 } else { i };
            let url = format!(
                "https://fubon-ebrokerdj.fbs.com.tw/z/zc/zco/zco_{}_{}.djhtm",
                stock, index
            );
            let crawl_url = format!("{}&url={}", proxy_url, url);

            url_tx.send(crawl_url).await.expect("Failed to send URL");
        }
    }

    drop(url_tx);
}

async fn fetch_urls(mut url_rx: mpsc::Receiver<String>, capacity: usize) {
    let semaphore = Arc::new(Semaphore::new(50));
    let (content_tx, content_rx) = mpsc::channel(capacity);

    let fetch_handle = tokio::spawn(async move {
        while let Some(url) = url_rx.recv().await {
            print!(".");
            io::stdout().flush().unwrap();

            let sem_clone = Arc::clone(&semaphore);
            let content_tx_clone = content_tx.clone();
            tokio::spawn(async move {
                let _permit = sem_clone
                    .acquire()
                    .await
                    .expect("Failed to acquire semaphore permit");

                match fetch_content(url.clone()).await {
                    Ok(payload) => {
                        print!("_");
                        io::stdout().flush().unwrap();

                        if let Err(e) = content_tx_clone.send(payload).await {
                            eprintln!("Failed to send content: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch content for URL {}: {}", url, e);
                        // continue to the next URL without sending anything to content_tx
                    }
                }
                // Permit is automatically released when dropped
            });
        }
    });

    let aggregate_handle = tokio::spawn(aggregate(content_rx));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(fetch_handle, aggregate_handle);
}

async fn aggregate(mut content_rx: mpsc::Receiver<Payload>) {
    let today = Local::now();
    let formatted_date = format!("{}{:02}{:02}", today.year(), today.month(), today.day());
    let mut stock_map: HashMap<String, Model> = HashMap::new();

    while let Some(payload) = content_rx.recv().await {
        let url = payload.source.clone();
        let parser = Parser::new(ConcentrationStrategy);
        let res = parser.parse(payload).await;
        print!("*");
        io::stdout().flush().unwrap();
        if let Ok(res_value) = res {
            let model = stock_map
                .entry(res_value.0.clone())
                .or_insert_with(|| Model {
                    stock_id: res_value.0,
                    exchange_date: formatted_date.clone(),
                    concentration: vec![0; 5],
                });
            model.concentration[res_value.1] = res_value.2;
        } else if let Err(e) = res {
            eprintln!("Failed to parse content for URL {}: {}", url, e);
        }
    }

    // extract items from map and print json string
    println!(" ");
    for (_, model) in stock_map.iter() {
        println!("{}", model.to_json().unwrap());
    }
}
