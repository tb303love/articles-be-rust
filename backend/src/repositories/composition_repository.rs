use crate::dtos::ComponentInputDto;
use crate::models::ArticleComposition;
use async_trait::async_trait;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

#[async_trait]
pub trait CompositionRepository: Send + Sync {
    async fn find_compositions_by_bundle_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<ArticleComposition>, Error>;
    async fn find_by_bundle_ids(&self, ids: &[i64]) -> Result<Vec<ArticleComposition>, Error>;
    async fn get_affected_bundle_names(&self, stock_id: i64) -> Result<Vec<String>, Error>;
    async fn save_compositions(
        &self,
        bundle_id: i64,
        comps: &[ComponentInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;
    async fn delete_compositions(
        &self,
        bundle_id: i64,
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;
}

pub struct SqlxCompositionRepository {
    pool: MySqlPool,
}

impl SqlxCompositionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CompositionRepository for SqlxCompositionRepository {
    async fn find_compositions_by_bundle_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<ArticleComposition>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, bundle_id, component_id, quantity FROM article_composition WHERE bundle_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");

        builder
            .build_query_as::<ArticleComposition>()
            .fetch_all(&self.pool) // Koristi interni pool za čitanje
            .await
    }

    async fn find_by_bundle_ids(&self, ids: &[i64]) -> Result<Vec<ArticleComposition>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new("SELECT id, bundle_id, component_id, quantity FROM article_composition WHERE bundle_id IN (");
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        builder
            .build_query_as::<ArticleComposition>()
            .fetch_all(&self.pool)
            .await
    }

    async fn get_affected_bundle_names(&self, stock_id: i64) -> Result<Vec<String>, Error> {
        sqlx::query_scalar::<_, String>(
            "SELECT a.name FROM article a INNER JOIN article_composition ac ON a.id = ac.bundle_id WHERE ac.component_id = ?"
        ).bind(stock_id).fetch_all(&self.pool).await
    }

    async fn save_compositions(
        &self,
        bundle_id: i64,
        comps: &[ComponentInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        if comps.is_empty() {
            return Ok(());
        }
        let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT INTO article_composition (bundle_id, component_id, quantity) ",
        );
        qb.push_values(comps, |mut b, comp| {
            b.push_bind(bundle_id)
                .push_bind(comp.component_id)
                .push_bind(comp.quantity);
        });
        qb.build().execute(tx).await?;
        Ok(())
    }

    async fn delete_compositions(
        &self,
        bundle_id: i64,
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_composition WHERE bundle_id = ?")
            .bind(bundle_id)
            .execute(tx)
            .await?;
        Ok(())
    }
}
