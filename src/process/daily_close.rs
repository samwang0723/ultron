use crate::engine::fetcher::fetch_content;
use crate::engine::parser::{DailyCloseStrategy, Parser};
use chrono::{Datelike, Local};

pub async fn execute() {
    let day = Local::now();
    let formatted_date = format!("{}{:02}{:02}", day.year(), day.month(), day.day());
    let url = format!(
        "https://www.twse.com.tw/exchangeReport/MI_INDEX?response=csv&date={}&type=ALLBUT0999",
        formatted_date
    );
    println!("Fetching data from {}", url);
    match fetch_content(url).await {
        Ok(payload) => {
            let mut cloned = payload.clone();
            cloned.date = Some(formatted_date);
            let parser = Parser::new(DailyCloseStrategy);
            match parser.parse(cloned).await {
                Ok(result) => {
                    for record in result {
                        println!("{}", record.to_json().unwrap());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse payload: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch payload: {}", e);
        }
    }
}
