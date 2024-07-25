use super::kafka::Producer;
use crate::config::setting::SETTINGS;
use crate::engine::fetcher::{fetch_content, Payload};
use crate::engine::models::concentration::Concentration;
use crate::engine::parser::Parser;
use crate::engine::strategies::concentration::ConcentrationStrategy;

use chrono::{Datelike, Local};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

static CONCENTRATION_PAGES: usize = 5;

pub async fn execute(stocks: Vec<String>) {
    let today = Local::now();
    let formatted_date = format!("{}{:02}{:02}", today.year(), today.month(), today.day());

    // If formatted date in array, skip
    let skipped_dates = vec![
        "20240101", "20240206", "20240207", "20240208", "20240209", "20240212", "20240213",
        "20240214", "20240228", "20240404", "20240405", "20240501", "20240610", "20240917",
        "20241010",
    ];
    if skipped_dates.contains(&formatted_date.as_str()) {
        println!("Skipped date: {}", formatted_date);
        return;
    }

    let capacity = stocks.len() * CONCENTRATION_PAGES;
    let (url_tx, url_rx) = mpsc::channel(capacity);

    // retrieve all handles and ensure process not termiated before tasks completed
    let url_gen_handle = tokio::spawn(generate_urls(url_tx, stocks.clone()));
    let fetch_aggregate_handle = tokio::spawn(fetch_urls(url_rx, capacity));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(url_gen_handle, fetch_aggregate_handle);
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

async fn fetch_urls(mut url_rx: mpsc::Receiver<String>, capacity: usize) {
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

    let aggregate_handle = tokio::spawn(aggregate(content_rx));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(fetch_handle, aggregate_handle);
}

async fn aggregate(mut content_rx: mpsc::Receiver<Payload>) {
    let today = Local::now();
    let formatted_date = format!("{}{:02}{:02}", today.year(), today.month(), today.day());
    let mut stock_map: HashMap<String, Concentration> = HashMap::new();
    // Create a new producer using match to handle the Result
    let kproducer = match Producer::new(&SETTINGS.kafka.connection_string()) {
        Ok(kproducer) => kproducer,
        Err(e) => {
            eprintln!("Failed to create producer: {}", e);
            return;
        }
    };

    while let Some(payload) = content_rx.recv().await {
        let url = payload.source.clone();
        let parser = Parser::new(ConcentrationStrategy);
        let res = parser.parse(payload).await;

        if let Ok(res_value) = res {
            let model = stock_map
                .entry(res_value.0.clone())
                .or_insert_with(|| Concentration {
                    stock_id: res_value.0,
                    exchange_date: formatted_date.clone(),
                    concentration: vec![0; 5],
                    sum_buy_shares: 0,
                    sum_sell_shares: 0,
                    avg_buy_price: 0.0,
                    avg_sell_price: 0.0,
                    current: 0,
                });
            model.concentration[res_value.1] = res_value.2; // Set diff num based on index

            if res_value.1 == 0 {
                model.sum_buy_shares = res_value.3;
                model.sum_sell_shares = res_value.4;
                model.avg_buy_price = res_value.5;
                model.avg_sell_price = res_value.6;
            }

            model.current += 1;
            if model.current == 5 {
                let payload = model.to_json().unwrap();
                match &kproducer
                    .send("stakeconcentration-v1".to_string(), payload.clone())
                    .await
                {
                    Ok(_) => println!("{}", payload),
                    Err(e) => eprintln!("Failed to send message: {}", e),
                }
            }
        } else if let Err(e) = res {
            eprintln!("Failed to parse content for URL {}: {}", url, e);
        }
    }
}
