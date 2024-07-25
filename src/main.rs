mod config;
mod engine;
mod process;
mod repository;

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use clap::Parser;
use config::setting::SETTINGS;
use repository::adapter::Adapter;
use sqlx::postgres::PgPoolOptions;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Data type of target
    #[arg(short, long)]
    target: String,

    /// Date in the format "YYYYMMDD"
    #[arg(short, long)]
    date: Option<String>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Read the settings
    let args = Args::parse();
    let date: DateTime<Local> = match args.date {
        Some(date_str) => {
            let from = NaiveDate::parse_from_str(&date_str, "%Y%m%d").unwrap();
            let dt = from.and_hms_opt(0, 0, 0).unwrap();
            Local.from_local_datetime(&dt).unwrap()
        }
        None => Local::now(),
    };

    match args.target.as_str() {
        "daily_close" => process::daily_close::execute(date).await,
        "three_primary" => process::three_primary::execute(date).await,
        "concentration" => {
            // Create a connection pool
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&SETTINGS.database.connection_string())
                .await
                .ok()
                .unwrap();

            let pg_adapter = Adapter::new(pool);
            let ids = pg_adapter.get_stock_ids().await.ok().unwrap();

            process::concentration::execute(ids).await;
        }
        target => eprintln!("Unknown target: {}", target),
    }
}
