use serde::Serialize;
use serde_json;

#[derive(Debug, Serialize)]
pub struct Model {
    pub stock_id: String,
    pub exchange_date: String,
    pub concentration: Vec<i32>,
}

impl Model {
    // Define a method to convert the struct into a JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug)]
pub struct Concentration(pub String, pub usize, pub i32);

// Testcases for Model
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_to_json() {
        let model = Model {
            stock_id: String::from("AAPL"),
            exchange_date: String::from("2020-01-01"),
            concentration: vec![1, 2, 3],
        };

        let json_string = model.to_json().unwrap();
        assert_eq!(
            json_string,
            r#"{"stock_id":"AAPL","exchange_date":"2020-01-01","concentration":[1,2,3]}"#
        );
    }
}
