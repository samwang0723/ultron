use super::adapter::DatabaseClient;
use async_trait::async_trait;
use sqlx::postgres::PgPool;
use sqlx::Error;

#[async_trait]
impl DatabaseClient for PgPool {
    async fn get_stock_ids(&self) -> Result<Vec<String>, Error> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM stocks WHERE deleted_at IS NULL")
            .fetch_all(self)
            .await?;
        let results: Vec<String> = rows.into_iter().map(|(id,)| id).collect();
        Ok(results)
    }
}
