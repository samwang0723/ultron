use anyhow::{anyhow, Result};
use async_trait::async_trait;
use csv::StringRecord;

use crate::engine::fetcher;
use crate::engine::models::*;
use crate::engine::parser::*;

#[derive(Debug)]
pub struct DailyCloseStrategy;

impl Conversion for DailyCloseStrategy {}

impl DailyCloseStrategy {
    fn is_valid_record(&self, record: &StringRecord, index_set: &daily_close::CsvIndexSet) -> bool {
        record.len() >= 17
            && self.is_integer(&record[index_set.stock_id])
            && [
                index_set.open,
                index_set.high,
                index_set.low,
                index_set.close,
                index_set.diff,
            ]
            .iter()
            .all(|&index| self.valid(&record[index]))
    }

    fn parse_record(
        &self,
        record: &StringRecord,
        index_set: &daily_close::CsvIndexSet,
        date: &Option<String>,
    ) -> Result<daily_close::DailyClose> {
        let diff = self.parse_with_comma::<f32>(&record[index_set.diff])?;
        let diff = match index_set.diff_sign {
            Some(index) if record[index].contains('-') => -diff,
            _ => diff,
        };

        Ok(daily_close::DailyClose {
            stock_id: record[index_set.stock_id].to_string(),
            exchange_date: date.clone().unwrap_or_default(),
            trade_shares: self.parse_with_comma::<i64>(&record[index_set.trade_shares])?,
            transactions: self.parse_with_comma::<i32>(&record[index_set.transactions])?,
            turnover: self.parse_with_comma::<i64>(&record[index_set.turnover])?,
            open: self.parse_with_comma::<f32>(&record[index_set.open])?,
            high: self.parse_with_comma::<f32>(&record[index_set.high])?,
            low: self.parse_with_comma::<f32>(&record[index_set.low])?,
            close: self.parse_with_comma::<f32>(&record[index_set.close])?,
            diff,
        })
    }

    fn is_integer(&self, s: &str) -> bool {
        s.parse::<i32>().is_ok() && s.len() == 4
    }

    fn valid(&self, s: &str) -> bool {
        !s.is_empty() && s != "---"
    }
}

#[async_trait]
impl ParseStrategy for DailyCloseStrategy {
    type Error = anyhow::Error;
    type Input = fetcher::Payload;
    type Output = Vec<daily_close::DailyClose>;

    async fn parse(&self, payload: Self::Input) -> Result<Self::Output, Self::Error> {
        let index_set = if payload.source.contains("twse") {
            daily_close::CsvIndexSet::new_twse()
        } else if payload.source.contains("tpex") {
            daily_close::CsvIndexSet::new_tpex()
        } else {
            return Err(anyhow!("Cannot identify parse index"));
        };

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b',')
            .flexible(true)
            .from_reader(payload.content.as_bytes());

        let records = rdr
            .records()
            .filter_map(|result| result.ok())
            .filter(|record| self.is_valid_record(record, &index_set))
            .filter_map(|record| self.parse_record(&record, &index_set, &payload.date).ok())
            .collect();

        Ok(records)
    }
}
