use crate::engine::fetcher::fetch_content;
use crate::engine::parser::{DailyCloseStrategy, Parser};
use chrono::{Datelike, Duration, Local};

pub async fn execute() {
    let day = Local::now() - Duration::days(1);
    let formatted_date = format!("{}{:02}{:02}", day.year(), day.month(), day.day());
    let url = format!(
        "https://www.twse.com.tw/exchangeReport/MI_INDEX?response=csv&date={}&type=ALLBUT0999",
        formatted_date
    );
    println!("Fetching data from {}", url);
    match fetch_content(url).await {
        Ok(payload) => {
            let parser = Parser::new(DailyCloseStrategy);
            match parser.parse(payload).await {
                Ok(result) => {
                    println!("{:?}", result);
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
