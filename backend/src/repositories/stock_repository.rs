use crate::dtos::ArticleStockInputDto;
use crate::models::ArticleStock;
use async_trait::async_trait;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

#[async_trait]
pub trait StockRepository: Send + Sync {
    async fn find_stocks_by_article_ids(&self, ids: &[i64]) -> Result<Vec<ArticleStock>, Error>;
    async fn find_by_article_ids(&self, ids: &[i64]) -> Result<Vec<ArticleStock>, Error>;
    async fn save_stocks(
        &self,
        article_id: i64,
        stocks: &[ArticleStockInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;
    async fn delete_stocks(&self, article_id: i64, tx: &mut MySqlConnection) -> Result<(), Error>;
}

pub struct SqlxStockRepository {
    pool: MySqlPool,
}

impl SqlxStockRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StockRepository for SqlxStockRepository {
    async fn find_stocks_by_article_ids(&self, ids: &[i64]) -> Result<Vec<ArticleStock>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, article_id, store_id, quantity FROM article_stock WHERE article_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");

        builder
            .build_query_as::<ArticleStock>()
            .fetch_all(&self.pool) // Koristi interni pool za čitanje
            .await
    }
    async fn find_by_article_ids(&self, ids: &[i64]) -> Result<Vec<ArticleStock>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, article_id, store_id, quantity FROM article_stock WHERE article_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        builder
            .build_query_as::<ArticleStock>()
            .fetch_all(&self.pool)
            .await
    }

    async fn save_stocks(
        &self,
        article_id: i64,
        stocks: &[ArticleStockInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        if stocks.is_empty() {
            return Ok(());
        }
        let mut qb: QueryBuilder<MySql> =
            QueryBuilder::new("INSERT INTO article_stock (article_id, store_id, quantity) ");
        qb.push_values(stocks, |mut b, stock| {
            b.push_bind(article_id)
                .push_bind(stock.store_id)
                .push_bind(&stock.quantity);
        });
        qb.build().execute(tx).await?;
        Ok(())
    }

    async fn delete_stocks(&self, article_id: i64, tx: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_stock WHERE article_id = ?")
            .bind(article_id)
            .execute(tx)
            .await?;
        Ok(())
    }
}
