use super::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use csv::StringRecord;
use models::concentration;
use models::daily_close;
use models::three_primary;
use regex::Regex;
use scraper::{Html, Selector};
use std::fmt::Display;
use std::str::FromStr;

#[async_trait]
pub trait ParseStrategy: Conversion {
    type Error;
    type Input;
    type Output; // Declare an associated type that will be the return type of the parse method.

    async fn parse(&self, payload: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub trait Conversion {
    fn parse_with_comma<T: FromStr>(&self, data: &str) -> Result<T>
    where
        T::Err: Display,
    {
        let without_comma = data.trim().replace(',', "");
        without_comma
            .parse::<T>()
            .map_err(|e| anyhow!("Failed to parse {}: {}", without_comma, e))
    }
}

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

#[derive(Debug)]
pub struct ConcentrationStrategy;

impl Conversion for ConcentrationStrategy {}

impl ConcentrationStrategy {
    fn identifier(&self, url: &str) -> Result<(String, usize), anyhow::Error> {
        let re = Regex::new(r"zco_(\d+)_(\d+)")?;
        let captures = re.captures(url).ok_or_else(|| anyhow!("Invalid URL"))?;

        let stock_id = captures.get(1).map_or("", |m| m.as_str());
        let index = captures
            .get(2)
            .map_or(Ok(0), |m| m.as_str().parse::<usize>())
            .map_err(|_| anyhow!("Failed to parse index"))?;

        // backfill the missing index 4 (40 days replaced with 60 days)
        let pos = if index == 6 { index - 2 } else { index - 1 };

        Ok((stock_id.to_string(), pos))
    }
}

#[async_trait]
impl ParseStrategy for ConcentrationStrategy {
    type Error = anyhow::Error;
    type Input = fetcher::Payload;
    type Output = concentration::Temp;

    async fn parse(&self, payload: Self::Input) -> Result<Self::Output, Self::Error> {
        let (stock_id, pos) = self.identifier(&payload.source)?;
        println!(
            "source: {}, content-type: {}",
            payload.source, payload.content_type
        );
        let document = Html::parse_document(&payload.content);
        let selector = Selector::parse("td.t3n1[colspan='4']")
            .map_err(|e| anyhow!("Failed to create selector: {}", e))?;

        let values = document
            .select(&selector)
            .filter_map(|element| element.text().next())
            .collect::<Vec<_>>();

        if values.len() != 4 {
            return Err(anyhow!("Expected 4 values, found {}", values.len()));
        }

        let total_buy = self.parse_with_comma::<i32>(values[0])?;
        let total_sell = self.parse_with_comma::<i32>(values[1])?;
        let avg_buy_price = self.parse_with_comma::<f32>(values[2])?;
        let avg_sell_price = self.parse_with_comma::<f32>(values[3])?;

        Ok(concentration::Temp(
            stock_id,
            pos,
            total_buy - total_sell,
            total_buy,
            total_sell,
            avg_buy_price,
            avg_sell_price,
        ))
    }
}

#[derive(Debug)]
pub struct Parser<T: ParseStrategy> {
    strategy: T,
}

impl<T: ParseStrategy> Parser<T> {
    pub fn new(strategy: T) -> Self {
        Self { strategy }
    }

    pub async fn parse(&self, payload: T::Input) -> Result<T::Output, T::Error> {
        self.strategy.parse(payload).await
    }
}

// Testcases for ConcentrationStrategy parse and to_i32, to_usize
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_concentration_strategy_parse() {
        let strategy = ConcentrationStrategy {};
        let payload = fetcher::Payload {
            date: None,
            content_type: "text/html".to_string(),
            source: "https://fubon-ebrokerdj.fbs.com.tw/z/zc/zco/zco_2330_2.djhtm".to_string(),
            content: r##"<table class="hasBorder" width="100%" cellspacing="1" cellpadding="0" border="0" bgcolor="#F0F0F0"><TR>
            <TD class="t4t1" nowrap><a href="/z/zc/zco/zco0/zco0.djhtm?a=3704&b=0035003300380045&BHID=5380">第一金-自由</a></TD>
            <TD class="t3n1">34</TD>
            <TD class="t3n1">8</TD>
            <TD class="t3n1">26</TD>
            <TD class="t3n1">0.32%</TD>
            <TD class="t4t1" nowrap><a href="/z/zc/zco/zco0/zco0.djhtm?a=3704&b=0039003200300041&BHID=9200">凱基-板橋</a></TD>
            <TD class="t3n1">2</TD>
            <TD class="t3n1">36</TD>
            <TD class="t3n1">34</TD>
            <TD class="t3n1">0.42%</TD>
            </tr>
            <TR id="oScrollFoot">
            <TD class="t4t1" nowrap>合計買超張數</td>
            <td class="t3n1" colspan=4>2,108</td>
            <TD class="t4t1" nowrap>合計賣超張數</td>
            <td class="t3n1" colspan=4>1,252</td>
            </TR>
            <TR id="oScrollFoot">
            <TD class="t4t1" nowrap>平均買超成本</td>
            <td class="t3n1" colspan=4>54.59</td>
            <TD class="t4t1" nowrap>平均賣超成本</td>
            <td class="t3n1" colspan=4>54.32</td>
            </TR>
            <TR id="oScrollFoot">
            <td class="t3t1" colspan=10>
            【註1】上述買賣超個股僅提供排序後的前15名券商，且未計入自營商部份。<BR>
            【註2】合計買超或賣超，為上述家數合計。<BR>
            【註3】平均買超或賣超成本，為上述家數合計買賣超金額/上述家數合計買賣超張數。
            </td>
            </TR></table>"##
            .to_string(),
        };

        let result = strategy.parse(payload).await;
        assert!(result.is_ok());
        let concentration = result.unwrap();
        assert_eq!(concentration.0, "2330");
        assert_eq!(concentration.1, 1);
        assert_eq!(concentration.2, 856);
        assert_eq!(concentration.3, 2108);
        assert_eq!(concentration.4, 1252);
        assert_eq!(concentration.5, 54.59);
        assert_eq!(concentration.6, 54.32);
    }

    #[test]
    fn test_concentration_strategy_to_i32() {
        let strategy = ConcentrationStrategy {};
        let result = strategy.parse_with_comma::<i32>("1,845,919");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1845919);
    }

    #[test]
    fn test_concentration_strategy_to_i64() {
        let strategy = ConcentrationStrategy {};
        let result = strategy.parse_with_comma::<i64>("1,845,919");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1845919);
    }

    #[test]
    fn test_concentration_strategy_to_f32() {
        let strategy = ConcentrationStrategy {};
        let result = strategy.parse_with_comma::<f32>("-1.05 ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -1.05);
    }

    #[test]
    fn test_concentration_strategy_to_usize() {
        let strategy = ConcentrationStrategy {};
        let result = strategy.parse_with_comma::<usize>("2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }
}
