use async_trait::async_trait;
use sqlx::Error;

#[async_trait]
pub trait DatabaseClient {
    async fn get_stock_ids(&self) -> Result<Vec<String>, Error>;
}

pub struct Adapter<C: DatabaseClient + Send + Sync> {
    client: C,
}

impl<C: DatabaseClient + Send + Sync> Adapter<C> {
    pub fn new(client: C) -> Self {
        Adapter { client }
    }

    pub async fn get_stock_ids(&self) -> Result<Vec<String>, Error> {
        self.client.get_stock_ids().await
    }
}
