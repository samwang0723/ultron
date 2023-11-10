mod engine;
mod stocks;
use crate::engine::fetcher::{fetch_content, Payload};
use crate::engine::model::Model;
use crate::engine::parser::{ConcentrationStrategy, Parser};

use chrono::{Datelike, Local};
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Semaphore};

static CONCENTRATION_PAGES: usize = 5;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // List of URLs to process
    let stocks = match stocks::get_stock_codes_from_file("stocks.json") {
        Ok(stocks) => stocks,
        Err(e) => {
            eprintln!("Failed to read stock codes from file: {}", e);
            return;
        }
    };

    let capacity = stocks.len() * CONCENTRATION_PAGES;
    let (url_tx, url_rx) = mpsc::channel(capacity);

    // Display progress
    let pb = ProgressBar::new(stocks.len().try_into().unwrap());
    let mpb = Arc::new(Mutex::new(pb));

    // retrieve all handles and ensure process not termiated before tasks completed
    let url_gen_handle = tokio::spawn(generate_urls(url_tx, stocks.clone()));
    let fetch_aggregate_handle = tokio::spawn(fetch_urls(url_rx, mpb.clone(), capacity));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(url_gen_handle, fetch_aggregate_handle);

    // End the progress bar
    mpb.lock().unwrap().finish();
}

async fn generate_urls(url_tx: mpsc::Sender<String>, stocks: Vec<String>) {
    let mut sum = 0;
    for stock in stocks.iter() {
        for i in 1..=CONCENTRATION_PAGES {
            // skip the 40 days calculation
            let index = if i == CONCENTRATION_PAGES { i + 1 } else { i };
            let url = format!(
                "https://fubon-ebrokerdj.fbs.com.tw/z/zc/zco/zco_{}_{}.djhtm",
                stock, index
            );
            url_tx.send(url).await.expect("Failed to send URL");
        }
        sum += CONCENTRATION_PAGES;
        if sum % 25 == 0 {
            // Sleep for a while
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    drop(url_tx);
}

async fn fetch_urls(
    mut url_rx: mpsc::Receiver<String>,
    pb: Arc<Mutex<ProgressBar>>,
    capacity: usize,
) {
    let semaphore = Arc::new(Semaphore::new(50));
    let (content_tx, content_rx) = mpsc::channel(capacity);

    let fetch_handle = tokio::spawn(async move {
        while let Some(url) = url_rx.recv().await {
            let sem_clone = Arc::clone(&semaphore);
            let content_tx_clone = content_tx.clone();
            tokio::spawn(async move {
                let _permit = sem_clone
                    .acquire()
                    .await
                    .expect("Failed to acquire semaphore permit");

                match fetch_content(url.clone()).await {
                    Ok(payload) => {
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

    let aggregate_handle = tokio::spawn(aggregate(content_rx, pb));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(fetch_handle, aggregate_handle);
}

async fn aggregate(mut content_rx: mpsc::Receiver<Payload>, pb: Arc<Mutex<ProgressBar>>) {
    let today = Local::now();
    let formatted_date = format!("{}{:02}{:02}", today.year(), today.month(), today.day());
    let mut stock_map: HashMap<String, Model> = HashMap::new();

    while let Some(payload) = content_rx.recv().await {
        let url = payload.source.clone();
        let parser = Parser::new(ConcentrationStrategy);
        let res = parser.parse(payload).await;

        // Update progress
        let pb_ref = pb.lock().unwrap();
        pb_ref.inc(1);

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
    for (_, model) in stock_map.iter() {
        println!("{}", model.to_json().unwrap());
    }
}
