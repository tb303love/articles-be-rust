use crate::models::Article;
use async_trait::async_trait;
use sqlx::{Error, MySqlConnection, MySqlPool};

#[async_trait]
pub trait ArticleRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<Article>, Error>;
    async fn search(&self, search_term: Option<String>) -> Result<Vec<Article>, Error>;
    async fn save(&self, article: &Article, tx: &mut MySqlConnection) -> Result<i64, Error>;
    async fn delete(&self, id: i64, tx: &mut MySqlConnection) -> Result<(), Error>;
    async fn check_name_exists(&self, name: &str) -> Result<bool, Error>;
    async fn check_name_exists_exclude(&self, name: &str, id: i64) -> Result<bool, Error>;
    async fn get_or_create_category(
        &self,
        category_text: &str,
        tx: &mut MySqlConnection,
    ) -> Result<i64, Error>;
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

    async fn get_or_create_category(
        &self,
        category_text: &str,
        tx: &mut MySqlConnection,
    ) -> Result<i64, sqlx::Error> {
        // KORAK 1: Osiguraj da naziv postoji u `category_name`
        sqlx::query("INSERT IGNORE INTO category_name (name) VALUES (?)")
            .bind(category_text)
            .execute(&mut *tx)
            .await?;

        // Uzmi `category_name_id`
        let category_name_id: i64 =
            sqlx::query_scalar("SELECT id FROM category_name WHERE name = ?")
                .bind(category_text)
                .fetch_one(&mut *tx)
                .await?;

        // KORAK 2: Osiguraj da postoji zapis u tabeli `category` za ovaj name_id
        // Napomena: Ovo podrazumeva da u tabeli `category` kolona `category_name_id` ima UNIQUE indeks
        sqlx::query("INSERT IGNORE INTO category (category_name_id) VALUES (?)")
            .bind(category_name_id)
            .execute(&mut *tx)
            .await?;

        // KORAK 3: Uzmi konačni `id` iz tabele `category` koji ide u artikal
        let category_id: i64 =
            sqlx::query_scalar("SELECT id FROM category WHERE category_name_id = ?")
                .bind(category_name_id)
                .fetch_one(&mut *tx)
                .await?;

        Ok(category_id)
    }
}
