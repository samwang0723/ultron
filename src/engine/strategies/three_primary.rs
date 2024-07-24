use anyhow::{anyhow, Result};
use async_trait::async_trait;
use csv::StringRecord;

use crate::engine::fetcher;
use crate::engine::models::*;
use crate::engine::parser::*;

#[derive(Debug)]
pub struct ThreePrimaryStrategy;

impl Conversion for ThreePrimaryStrategy {}

impl ThreePrimaryStrategy {
    fn is_valid_record(
        &self,
        record: &StringRecord,
        index_set: &three_primary::CsvIndexSet,
    ) -> bool {
        record.len() >= 19
            && self.is_integer(&record[index_set.stock_id])
            && [
                index_set.foreign_trade_shares,
                index_set.trust_trade_shares,
                index_set.dealer_trade_shares,
                index_set.hedging_trade_shares,
            ]
            .iter()
            .all(|&index| self.valid(&record[index]))
    }

    fn parse_record(
        &self,
        record: &StringRecord,
        index_set: &three_primary::CsvIndexSet,
        date: &Option<String>,
    ) -> Result<three_primary::ThreePrimary> {
        Ok(three_primary::ThreePrimary {
            stock_id: record[index_set.stock_id].to_string(),
            exchange_date: date.clone().unwrap_or_default(),
            foreign_trade_shares: self
                .parse_with_comma::<i64>(&record[index_set.foreign_trade_shares])?,
            trust_trade_shares: self
                .parse_with_comma::<i64>(&record[index_set.trust_trade_shares])?,
            dealer_trade_shares: self
                .parse_with_comma::<i64>(&record[index_set.dealer_trade_shares])?,
            hedging_trade_shares: self
                .parse_with_comma::<i64>(&record[index_set.hedging_trade_shares])?,
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
impl ParseStrategy for ThreePrimaryStrategy {
    type Error = anyhow::Error;
    type Input = fetcher::Payload;
    type Output = Vec<three_primary::ThreePrimary>;

    async fn parse(&self, payload: Self::Input) -> Result<Self::Output, Self::Error> {
        let index_set = if payload.source.contains("twse") {
            three_primary::CsvIndexSet::new_twse()
        } else if payload.source.contains("tpex") {
            three_primary::CsvIndexSet::new_tpex()
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
