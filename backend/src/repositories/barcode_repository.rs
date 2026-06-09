use async_trait::async_trait;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

#[async_trait]
pub trait BarcodeRepository: Send + Sync {
    async fn find_barcodes_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error>;
    async fn find_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error>;
    async fn find_id_by_barcode(&self, barcode: &str) -> Result<i64, Error>;
    async fn save_barcodes(&self, article_id: i64, barcodes: &Option<Vec<String>>, tx: &mut MySqlConnection) -> Result<(), Error>;
    async fn delete_barcodes(&self, article_id: i64, tx: &mut MySqlConnection) -> Result<(), Error>;

}

pub struct SqlxBarcodeRepository { pool: MySqlPool }

impl SqlxBarcodeRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BarcodeRepository for SqlxBarcodeRepository {
    async fn find_barcodes_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT article_id, barcode FROM article_barcodes WHERE article_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");

        builder
            .build_query_as::<(i64, String)>()
            .fetch_all(&self.pool) // Koristi interni pool za čitanje
            .await
    }
    async fn find_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error> {
        if ids.is_empty() { return Ok(Vec::new()); }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new("SELECT article_id, barcode FROM article_barcodes WHERE article_id IN (");
        let mut sep = builder.separated(", ");
        for id in ids { sep.push_bind(*id); }
        sep.push_unseparated(")");
        builder.build_query_as::<(i64, String)>().fetch_all(&self.pool).await
    }

    async fn find_id_by_barcode(&self, barcode: &str) -> Result<i64, Error> {
        sqlx::query_scalar::<_, i64>("SELECT article_id FROM article_barcodes WHERE barcode = ?")
            .bind(barcode).fetch_one(&self.pool).await
    }

    async fn save_barcodes(&self, article_id: i64, barcodes: &Option<Vec<String>>, tx: &mut MySqlConnection) -> Result<(), Error> {
        if let Some(barcode_list) = barcodes {
            if !barcode_list.is_empty() {
                let mut qb: QueryBuilder<MySql> = QueryBuilder::new("INSERT INTO article_barcodes (article_id, barcode) ");
                qb.push_values(barcode_list, |mut b, barcode| {
                    b.push_bind(article_id).push_bind(barcode);
                });
                qb.build().execute(tx).await?;
            }
        }
        Ok(())
    }

    async fn delete_barcodes(&self, article_id: i64, tx: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_barcodes WHERE article_id = ?").bind(article_id).execute(tx).await?;
        Ok(())
    }
}
