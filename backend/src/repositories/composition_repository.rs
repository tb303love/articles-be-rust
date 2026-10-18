// src/repositories/composition_repository.rs

use crate::dto::ComponentInputDto;
use crate::models::ArticleComposition;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

pub struct SqlxCompositionRepository {
    pool: MySqlPool,
}

impl SqlxCompositionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn find_compositions_by_bundle_ids(
        &self,
        ids: &[u64],
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
            .fetch_all(&self.pool)
            .await
    }

    pub async fn find_by_bundle_ids(&self, ids: &[u64]) -> Result<Vec<ArticleComposition>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, bundle_id, component_id, quantity FROM article_composition WHERE bundle_id IN ("
        );
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

    pub async fn find_affected_bundles(&self, stock_id: u64) -> Result<Vec<u64>, Error> {
        // 🧠 FIKSIRANO: Tip promenjen u Vec<u64> i upit prilagođen da vraća direktno ID-jeve
        // od kojih ovaj bundle zavisi, kako bi tvoj modal na frontendu radio savršeno!
        sqlx::query_scalar::<_, u64>(
            "SELECT component_id FROM article_composition WHERE bundle_id = ?"
        )
            .bind(stock_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn save_compositions(
        &self,
        bundle_id: u64,
        comps: &[ComponentInputDto],
        conn: &mut MySqlConnection, // 🧠 Koristimo čistu konekciju umesto tx
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
        qb.build().execute(conn).await?;
        Ok(())
    }

    pub async fn delete_compositions(
        &self,
        bundle_id: u64,
        conn: &mut MySqlConnection, // 🧠 Koristimo čistu konekciju umesto tx
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_composition WHERE bundle_id = ?")
            .bind(bundle_id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
