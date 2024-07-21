use super::kafka::Producer;
use crate::config::setting::SETTINGS;
use crate::engine::fetcher::{fetch_content, Payload};
use crate::engine::parser::{Parser, ThreePrimaryStrategy};

use chrono::{DateTime, Datelike, Local};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

static CAPACITY: usize = 2;

pub async fn execute(d: DateTime<Local>) {
    let (url_tx, url_rx) = mpsc::channel(CAPACITY);

    // retrieve all handles and ensure process not termiated before tasks completed
    let url_gen_handle = tokio::spawn(generate_urls(d, url_tx));
    let fetch_aggregate_handle = tokio::spawn(fetch_urls(d, url_rx, CAPACITY));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(url_gen_handle, fetch_aggregate_handle);
}

fn get_date(day: DateTime<Local>, t: &str) -> String {
    match t {
        "twse" => format!("{}{:02}{:02}", day.year(), day.month(), day.day()),
        "tpex" => format!("{}/{:02}/{:02}", day.year() - 1911, day.month(), day.day()),
        _ => "".to_string(),
    }
}

async fn generate_urls(d: DateTime<Local>, url_tx: mpsc::Sender<String>) {
    let twse_url = format!(
        "https://www.twse.com.tw/rwd/zh/fund/T86?response=csv&date={}&selectType=ALLBUT0999",
        get_date(d, "twse")
    );

    let tpex_url = format!(
        "https://www.tpex.org.tw/web/stock/3insti/daily_trade/3itrade_hedge_result.php?l=zh-tw&o=csv&se=EW&t=D&d={}",
        get_date(d, "tpex")
    );

    let urls: [&str; 2] = [&twse_url, &tpex_url];
    for url in urls {
        url_tx
            .send(url.to_string())
            .await
            .expect("Failed to send URL");
    }

    drop(url_tx);
}

async fn fetch_urls(d: DateTime<Local>, mut url_rx: mpsc::Receiver<String>, capacity: usize) {
    let semaphore = Arc::new(Semaphore::new(2));
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

                println!("Fetching data from {}", url);
                match fetch_content(url, false).await {
                    Ok(payload) => {
                        if let Err(e) = content_tx_clone.send(payload).await {
                            eprintln!("Failed to send content: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch payload: {}", e);
                    }
                }
            });
        }
    });

    let aggregate_handle = tokio::spawn(aggregate(d, content_rx));
    // Await on both handles to ensure completion
    let _results = tokio::try_join!(fetch_handle, aggregate_handle);
}

async fn aggregate(d: DateTime<Local>, mut content_rx: mpsc::Receiver<Payload>) {
    let kproducer = Producer::new(&SETTINGS.kafka.brokers);

    while let Some(payload) = content_rx.recv().await {
        let mut cloned = payload.clone();
        cloned.date = Some(get_date(d, "twse"));
        let parser = Parser::new(ThreePrimaryStrategy);
        match parser.parse(cloned).await {
            Ok(result) => {
                for record in result {
                    let payload = record.to_json().unwrap();
                    match &kproducer
                        .send("threeprimary-v1".to_string(), payload.clone())
                        .await
                    {
                        Ok(_) => println!("{}", payload),
                        Err(e) => eprintln!("Failed to send message: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse payload: {}", e);
            }
        }
    }
}
