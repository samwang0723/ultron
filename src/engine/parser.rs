use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use scraper::{Html, Selector};

use super::*;

#[async_trait]
pub trait ParseStrategy: Conversion {
    type Error;
    type Input;
    // Declare an associated type that will be the return type of the parse method.
    type Output;

    async fn parse(&self, payload: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub trait Conversion {
    fn to_i32(&self, data: &str) -> Result<i32, anyhow::Error>;
    fn to_usize(&self, data: &str) -> Result<usize, anyhow::Error>;
}

#[derive(Debug)]
pub struct DailyCloseStrategy;

#[async_trait]
impl ParseStrategy for DailyCloseStrategy {
    type Error = anyhow::Error;
    type Input = String;
    type Output = String;

    async fn parse(&self, _payload: Self::Input) -> Result<Self::Output, Self::Error> {
        Err(anyhow!("DailyCloseStrategy parse not yet implemented"))
    }
}

impl Conversion for DailyCloseStrategy {
    fn to_i32(&self, _data: &str) -> Result<i32, anyhow::Error> {
        Err(anyhow!("DailyCloseStrategy to_i32 not yet implemented"))
    }

    fn to_usize(&self, _data: &str) -> Result<usize, anyhow::Error> {
        Err(anyhow!("DailyCloseStrategy to_usize not yet implemented"))
    }
}

#[derive(Debug)]
pub struct ConcentrationStrategy;

impl ConcentrationStrategy {
    fn identifier(&self, url: String) -> Result<(String, usize), anyhow::Error> {
        let re = Regex::new(r"zco_(\d+)_(\d+)")?;
        let captures = match re.captures(&url) {
            Some(captures) => captures,
            None => {
                return Err(anyhow!("Invalid URL"));
            }
        };

        let stock_id = captures.get(1).map_or("", |m| m.as_str());
        let index = self.to_usize(captures.get(2).map_or("", |m| m.as_str()))?;

        // backfill the missing index 4 (40 days replaced with 60 days)
        let pos = if index == 6 { index - 2 } else { index - 1 };

        Ok((stock_id.to_string(), pos))
    }
}

#[async_trait]
impl ParseStrategy for ConcentrationStrategy {
    type Error = anyhow::Error;
    type Input = fetcher::Payload;
    type Output = model::Concentration;

    async fn parse(&self, payload: Self::Input) -> Result<Self::Output, Self::Error> {
        let (stock_id, pos) = match self.identifier(payload.source.clone()) {
            Ok((stock_id, pos)) => (stock_id, pos),
            Err(e) => {
                return Err(anyhow!("Failed to parse URL: {}", e));
            }
        };

        let document = Html::parse_document(payload.content.as_str());
        let selector = match Selector::parse("td.t3n1[colspan]") {
            Ok(selector) => selector,
            Err(e) => {
                return Err(anyhow!("Failed to create selector: {}", e));
            }
        };

        let mut index: usize = 0;
        let mut total_buy = 0;
        let mut total_sell = 0;
        for element in document.select(&selector) {
            if let Some(colspan) = element.value().attr("colspan") {
                if colspan != "4" {
                    continue;
                }
            }
            let text = element.text().collect::<Vec<_>>().join("");
            match index {
                0 => total_buy = self.to_i32(&text)?,
                1 => total_sell = self.to_i32(&text)?,
                _ => {}
            }
            index += 1;
        }

        Ok(model::Concentration(stock_id, pos, total_buy - total_sell))
    }
}

impl Conversion for ConcentrationStrategy {
    fn to_i32(&self, data: &str) -> Result<i32, anyhow::Error> {
        let without_comma = data.replace(',', ""); // This will do nothing if there is no comma
        without_comma.parse::<i32>().map_err(|e| anyhow!(e))
    }

    fn to_usize(&self, data: &str) -> Result<usize, anyhow::Error> {
        data.parse::<usize>().map_err(|e| anyhow!(e))
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
    }

    #[test]
    fn test_concentration_strategy_to_i32() {
        let strategy = ConcentrationStrategy {};
        let result = strategy.to_i32("1,845,919");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1845919);
    }

    #[test]
    fn test_concentration_strategy_to_usize() {
        let strategy = ConcentrationStrategy {};
        let result = strategy.to_usize("2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }
}
