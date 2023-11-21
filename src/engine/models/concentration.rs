use serde::Serialize;
use serde_json;

#[derive(Debug, Serialize)]
pub struct Concentration {
    #[serde(rename = "stockId")]
    pub stock_id: String,

    #[serde(rename = "exchangeDate")]
    pub exchange_date: String,

    #[serde(rename = "diff")]
    pub concentration: Vec<i32>,

    #[serde(rename = "sumBuyShares")]
    pub sum_buy_shares: i32,

    #[serde(rename = "sumSellShares")]
    pub sum_sell_shares: i32,

    #[serde(rename = "avgBuyPrice")]
    pub avg_buy_price: f32,

    #[serde(rename = "avgSellPrice")]
    pub avg_sell_price: f32,

    #[serde(skip_serializing)]
    pub current: usize,
}

impl Concentration {
    // Define a method to convert the struct into a JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug)]
pub struct Temp(
    pub String,
    pub usize,
    pub i32,
    pub i32,
    pub i32,
    pub f32,
    pub f32,
);

// Testcases for Model
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_to_json() {
        let model = Concentration {
            stock_id: String::from("AAPL"),
            exchange_date: String::from("2020-01-01"),
            concentration: vec![1, 2, 3],
            sum_buy_shares: 0,
            sum_sell_shares: 0,
            avg_buy_price: 0.0,
            avg_sell_price: 0.0,
            current: 0,
        };

        let json_string = model.to_json().unwrap();
        assert_eq!(
            json_string,
            r#"{"stockId":"AAPL","exchangeDate":"2020-01-01","diff":[1,2,3],"sumBuyShares":0,"sumSellShares":0,"avgBuyPrice":0.0,"avgSellPrice":0.0}"#
        );
    }
}
