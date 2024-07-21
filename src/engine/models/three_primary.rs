use serde::Serialize;
use serde_json;

pub struct CsvIndexSet {
    pub stock_id: usize,
    pub foreign_trade_shares: usize,
    pub trust_trade_shares: usize,
    pub dealer_trade_shares: usize,
    pub hedging_trade_shares: usize,
}

impl CsvIndexSet {
    pub fn new_twse() -> Self {
        Self {
            stock_id: 0,
            foreign_trade_shares: 4,
            trust_trade_shares: 10,
            dealer_trade_shares: 14,
            hedging_trade_shares: 17,
        }
    }

    pub fn new_tpex() -> Self {
        Self {
            stock_id: 0,
            foreign_trade_shares: 10,
            trust_trade_shares: 13,
            dealer_trade_shares: 16,
            hedging_trade_shares: 19,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ThreePrimary {
    #[serde(rename = "stockId")]
    pub stock_id: String,

    #[serde(rename = "date")]
    pub exchange_date: String,

    #[serde(rename = "foreignTradeShares")]
    pub foreign_trade_shares: i64,

    #[serde(rename = "trustTradeShares")]
    pub trust_trade_shares: i64,

    #[serde(rename = "dealerTradeShares")]
    pub dealer_trade_shares: i64,

    #[serde(rename = "hedgingTradeShares")]
    pub hedging_trade_shares: i64,
}

impl ThreePrimary {
    // Define a method to convert the struct into a JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

// Testcases for Model
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_to_json() {
        let model = ThreePrimary {
            stock_id: String::from("AAPL"),
            exchange_date: String::from("2020-01-01"),
            foreign_trade_shares: 0,
            trust_trade_shares: 0,
            dealer_trade_shares: 0,
            hedging_trade_shares: 0,
        };

        let json_string = model.to_json().unwrap();
        assert_eq!(
            json_string,
            r#"{"stockId":"AAPL","date":"2020-01-01","foreignTradeShares":0,"trustTradeShares":0,"dealerTradeShares":0,"hedgingTradeShares":0}"#
        );
    }
}
