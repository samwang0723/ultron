use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::fmt::Display;
use std::str::FromStr;

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
