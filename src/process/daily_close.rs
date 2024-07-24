use crate::config::setting::SETTINGS;
use crate::engine::fetcher::{fetch_content, Payload};
use crate::engine::parser::Parser;
use crate::engine::strategies::daily_close::DailyCloseStrategy;
use crate::process::kafka::Producer;

use chrono::{DateTime, Datelike, Local};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

static CAPACITY: usize = 2;

pub async fn execute(date: DateTime<Local>) {
    let (url_tx, url_rx) = mpsc::channel(CAPACITY);

    // retrieve all handles and ensure process not termiated before tasks completed
    let url_gen_handle = tokio::spawn(generate_urls(date, url_tx));
    let fetch_aggregate_handle = tokio::spawn(fetch_urls(date, url_rx, CAPACITY));

    // Await on both handles to ensure completion
    let _results = tokio::try_join!(url_gen_handle, fetch_aggregate_handle);
}

fn get_date(day: DateTime<Local>, exchange_type: &str) -> String {
    match exchange_type {
        "twse" => format!("{}{:02}{:02}", day.year(), day.month(), day.day()),
        "tpex" => format!("{}/{:02}/{:02}", day.year() - 1911, day.month(), day.day()),
        _ => "".to_string(),
    }
}

async fn generate_urls(date: DateTime<Local>, url_tx: mpsc::Sender<String>) {
    let twse_url = format!(
        "https://www.twse.com.tw/exchangeReport/MI_INDEX?response=csv&date={}&type=ALLBUT0999",
        get_date(date, "twse")
    );

    let tpex_url = format!(
        "https://www.tpex.org.tw/web/stock/aftertrading/daily_close_quotes/stk_quote_download.php?l=zh-tw&d={}&s=0,asc,0",
        get_date(date, "tpex")
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

async fn fetch_urls(date: DateTime<Local>, mut url_rx: mpsc::Receiver<String>, capacity: usize) {
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
                match fetch_content(url).await {
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

    let aggregate_handle = tokio::spawn(aggregate(date, content_rx));
    // Await on both handles to ensure completion
    let _results = tokio::try_join!(fetch_handle, aggregate_handle);
}

async fn aggregate(date: DateTime<Local>, mut content_rx: mpsc::Receiver<Payload>) {
    let kproducer = Producer::new(&SETTINGS.kafka.connection_string());

    while let Some(raw) = content_rx.recv().await {
        let mut raw_payload = raw.clone();
        raw_payload.date = Some(get_date(date, "twse"));
        let parser = Parser::new(DailyCloseStrategy);
        match parser.parse(raw_payload).await {
            Ok(result) => {
                // print result
                println!("Parsed result: {:?}", result);

                for record in result {
                    match record.to_json() {
                        Ok(payload) => {
                            match &kproducer
                                .send("dailycloses-v1".to_string(), payload.clone())
                                .await
                            {
                                Ok(_) => println!("{}", payload),
                                Err(e) => eprintln!("Failed to send message: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Failed to convert record to JSON: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse payload: {}", e);
            }
        }
    }
}
