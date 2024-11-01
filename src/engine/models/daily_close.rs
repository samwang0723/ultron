use serde::Serialize;
use serde_json;

pub struct CsvIndexSet {
    pub stock_id: usize,
    pub trade_shares: usize,
    pub transactions: usize,
    pub turnover: usize,
    pub open: usize,
    pub high: usize,
    pub low: usize,
    pub close: usize,
    pub diff: usize,
    pub diff_sign: Option<usize>,
}

impl CsvIndexSet {
    pub fn new_twse() -> Self {
        Self {
            stock_id: 0,
            trade_shares: 2,
            transactions: 3,
            turnover: 4,
            open: 5,
            high: 6,
            low: 7,
            close: 8,
            diff: 10,
            diff_sign: Some(9),
        }
    }

    pub fn new_tpex() -> Self {
        Self {
            stock_id: 0,
            trade_shares: 7,
            transactions: 9,
            turnover: 8,
            open: 4,
            high: 5,
            low: 6,
            close: 2,
            diff: 3,
            diff_sign: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DailyClose {
    #[serde(rename = "stockId")]
    pub stock_id: String,

    #[serde(rename = "date")]
    pub exchange_date: String,

    #[serde(rename = "tradeShares")]
    pub trade_shares: i64,

    #[serde(rename = "transactions")]
    pub transactions: i32,

    #[serde(rename = "turnover")]
    pub turnover: i64,

    #[serde(rename = "open")]
    pub open: f32,

    #[serde(rename = "close")]
    pub close: f32,

    #[serde(rename = "high")]
    pub high: f32,

    #[serde(rename = "low")]
    pub low: f32,

    #[serde(rename = "priceDiff")]
    pub diff: f32,
}

impl DailyClose {
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
        let model = DailyClose {
            stock_id: String::from("AAPL"),
            exchange_date: String::from("2020-01-01"),
            trade_shares: 0,
            transactions: 0,
            turnover: 0,
            open: 0.0,
            close: 0.0,
            high: 0.0,
            low: 0.0,
            diff: 0.0,
        };

        let json_string = model.to_json().unwrap();
        assert_eq!(
            json_string,
            r#"{"stockId":"AAPL","date":"2020-01-01","tradeShares":0,"transactions":0,"turnover":0,"open":0.0,"close":0.0,"high":0.0,"low":0.0,"priceDiff":0.0}"#
        );
    }
}
