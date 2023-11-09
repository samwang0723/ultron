use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

// Define the structures corresponding to the JSON format
#[derive(Serialize, Deserialize, Debug)]
struct Stock {
    #[serde(rename = "stockID")]
    stock_id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct StockList {
    entries: Vec<Stock>,
}

// Function to read stock codes from a JSON file
pub fn get_stock_codes_from_file(file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    // Read the file
    let data = fs::read_to_string(file_path)?;

    // Deserialize the JSON data
    let stock_list: StockList = serde_json::from_str(&data)?;

    // Extract stock codes
    let stock_codes = stock_list
        .entries
        .into_iter()
        .map(|stock| stock.stock_id)
        .collect();

    Ok(stock_codes)
}
