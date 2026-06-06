// src/repositories/article_repository.rs (Deo 1)
use crate::dtos::{ArticleStockInputDto, ComponentInputDto};
use crate::models::{Article, ArticleComposition, ArticleStock};
use async_trait::async_trait;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

#[async_trait]
pub trait ArticleRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Article>, Error>;
    async fn search(&self, search_term: Option<String>) -> Result<Vec<Article>, Error>;
    async fn save(&self, article: &Article, tx: &mut MySqlConnection) -> Result<i64, Error>;
    async fn delete(&self, id: i64, tx: &mut MySqlConnection) -> Result<(), Error>;

    async fn check_name_exists(&self, name: &str) -> Result<bool, Error>;
    async fn check_name_exists_exclude(&self, name: &str, id: i64) -> Result<bool, Error>;
    async fn find_id_by_barcode(&self, barcode: &str) -> Result<i64, Error>;
    async fn get_affected_bundle_names(&self, stock_id: i64) -> Result<Vec<String>, Error>;

    async fn find_barcodes_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error>;
    async fn find_stocks_by_article_ids(&self, ids: &[i64]) -> Result<Vec<ArticleStock>, Error>;
    async fn find_compositions_by_bundle_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<ArticleComposition>, Error>;

    async fn save_barcodes(
        &self,
        article_id: i64,
        barcodes: &[String],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;
    async fn save_compositions(
        &self,
        bundle_id: i64,
        comps: &[ComponentInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;
    async fn save_stocks(
        &self,
        article_id: i64,
        stocks: &[ArticleStockInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;

    async fn delete_barcodes(&self, article_id: i64, tx: &mut MySqlConnection)
        -> Result<(), Error>;
    async fn delete_stocks(&self, article_id: i64, tx: &mut MySqlConnection) -> Result<(), Error>;
    async fn delete_compositions(
        &self,
        bundle_id: i64,
        tx: &mut MySqlConnection,
    ) -> Result<(), Error>;
}

pub struct SqlxArticleRepository {
    pool: MySqlPool,
}

impl SqlxArticleRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ArticleRepository for SqlxArticleRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Article>, Error> {
        let row = sqlx::query_as::<_, Article>(
            "SELECT id, name, price, admission_price1, admission_price2, category_id, article_image_id, active \
             FROM article WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn search(&self, search_term: Option<String>) -> Result<Vec<Article>, Error> {
        let query_str = match search_term {
            Some(term) => format!("%{}%", term.to_lowercase()),
            None => "%".to_string(),
        };

        let articles = sqlx::query_as::<_, Article>(
            "SELECT DISTINCT a.* FROM article a \
             LEFT JOIN article_barcodes b ON a.id = b.article_id \
             WHERE LOWER(a.name) LIKE ? OR b.barcode = ? ORDER BY a.name ASC",
        )
        .bind(&query_str)
        .bind(&query_str)
        .fetch_all(&self.pool)
        .await?;
        Ok(articles)
    }

    async fn check_name_exists(&self, name: &str) -> Result<bool, Error> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM article WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    async fn check_name_exists_exclude(&self, name: &str, id: i64) -> Result<bool, Error> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM article WHERE name = ? AND id != ?")
                .bind(name)
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }

    async fn find_id_by_barcode(&self, barcode: &str) -> Result<i64, Error> {
        let id = sqlx::query_scalar::<_, i64>(
            "SELECT article_id FROM article_barcodes WHERE barcode = ?",
        )
        .bind(barcode)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn get_affected_bundle_names(&self, stock_id: i64) -> Result<Vec<String>, Error> {
        let names = sqlx::query_scalar::<_, String>(
            "SELECT a.name FROM article a \
             INNER JOIN article_composition ac ON a.id = ac.bundle_id \
             WHERE ac.component_id = ?",
        )
        .bind(stock_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(names)
    }
    // src/repositories/article_repository.rs (Deo 2)

    // --- MASOVNO UČITAVANJE (Bulk Fetch) ---

    async fn find_barcodes_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
            "SELECT article_id, barcode FROM article_barcodes WHERE article_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");

        builder
            .build_query_as::<(i64, String)>()
            .fetch_all(&self.pool)
            .await
    }

    async fn find_stocks_by_article_ids(&self, ids: &[i64]) -> Result<Vec<ArticleStock>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
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

    async fn find_compositions_by_bundle_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<ArticleComposition>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
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

    // --- STRATEGIJA UPISA (CRUD & Bulk Write) ---

    async fn save(&self, article: &Article, tx: &mut MySqlConnection) -> Result<i64, Error> {
        if let Some(id) = article.id {
            sqlx::query(
                    "UPDATE article SET name = ?, price = ?, admission_price1 = ?, admission_price2 = ?, active = ?, category_id = ?, article_image_id = ? WHERE id = ?"
                )
                .bind(&article.name).bind(&article.price).bind(&article.admission_price_1)
                .bind(&article.admission_price_2).bind(article.active).bind(article.category_id)
                .bind(article.article_image_id).bind(id)
                .execute(tx).await?;
            Ok(id)
        } else {
            let res = sqlx::query(
                    "INSERT INTO article (name, price, admission_price1, admission_price2, active, category_id, article_image_id) VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&article.name).bind(&article.price).bind(&article.admission_price_1)
                .bind(&article.admission_price_2).bind(article.active).bind(article.category_id)
                .bind(article.article_image_id)
                .execute(tx).await?;
            Ok(res.last_insert_id() as i64)
        }
    }

    async fn delete(&self, id: i64, tx: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM article WHERE id = ?")
            .bind(id)
            .execute(tx)
            .await?;
        Ok(())
    }

    async fn save_barcodes(
        &self,
        article_id: i64,
        barcodes: &[String],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        if barcodes.is_empty() {
            return Ok(());
        }
        let mut qb: QueryBuilder<'_, MySql> =
            QueryBuilder::new("INSERT INTO article_barcodes (article_id, barcode) ");
        qb.push_values(barcodes, |mut b, barcode| {
            b.push_bind(article_id).push_bind(barcode);
        });
        qb.build().execute(tx).await?;
        Ok(())
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
        let mut qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
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

    async fn save_stocks(
        &self,
        article_id: i64,
        stocks: &[ArticleStockInputDto],
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        if stocks.is_empty() {
            return Ok(());
        }
        let mut qb: QueryBuilder<'_, MySql> =
            QueryBuilder::new("INSERT INTO article_stock (article_id, store_id, quantity) ");
        qb.push_values(stocks, |mut b, s| {
            b.push_bind(article_id)
                .push_bind(s.store_id)
                .push_bind(s.quantity);
        });
        qb.build().execute(tx).await?;
        Ok(())
    }

    // --- POJEDINAČNA ČIŠĆENJA RELACIJA (Orphan Removal) ---

    async fn delete_barcodes(
        &self,
        article_id: i64,
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_barcodes WHERE article_id = ?")
            .bind(article_id)
            .execute(tx)
            .await?;
        Ok(())
    }

    async fn delete_stocks(&self, article_id: i64, tx: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_stock WHERE article_id = ?")
            .bind(article_id)
            .execute(tx)
            .await?;
        Ok(())
    }

    async fn delete_compositions(
        &self,
        bundle_id: i64,
        tx: &mut MySqlConnection,
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_composition WHERE bundle_id = ? OR component_id = ?")
            .bind(bundle_id)
            .bind(bundle_id)
            .execute(tx)
            .await?;
        Ok(())
    }
} // Kraj impl bloka
