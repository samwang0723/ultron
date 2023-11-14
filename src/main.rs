mod engine;
mod process;
mod stocks;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Data type of target
    #[arg(short, long)]
    target: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    match args.target.as_str() {
        "daily_close" => {
            // Implement your logic for daily_close here
        }
        "concentration" => {
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
        target => eprintln!("Unknown target: {}", target),
    }
}
