mod engine;
mod kafka;
mod process;
mod stocks;

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

    process::concentration::execute(stocks).await;
}
