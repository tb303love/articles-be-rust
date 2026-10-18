use crate::models::Article;
use sqlx::{Error, MySqlConnection, MySqlPool};

pub struct SqlxArticleRepository {
    pool: MySqlPool,
}

impl SqlxArticleRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: u64) -> Result<Option<Article>, Error> {
        let row = sqlx::query_as::<_, Article>(
            "SELECT id, name, price, admission_price1, admission_price2, category_id, article_image_id, active FROM article WHERE id = ?"
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn search(&self, search_term: Option<String>) -> Result<Vec<Article>, Error> {
        let clean_term = search_term.unwrap_or_default().trim().to_string();
        if clean_term.is_empty() {
            let articles = sqlx::query_as::<_, Article>(
                "SELECT DISTINCT id, name, price, admission_price1, admission_price2, category_id, article_image_id, active FROM article ORDER BY name ASC"
            )
                .fetch_all(&self.pool)
                .await?;
            return Ok(articles);
        }

        let like_str = format!("%{}%", clean_term.to_lowercase());
        let articles = sqlx::query_as::<_, Article>(
            "SELECT DISTINCT a.id, a.name, a.price, a.admission_price1, a.admission_price2, a.category_id, a.article_image_id, a.active FROM article a \
             LEFT JOIN article_barcodes b ON a.id = b.article_id \
             WHERE LOWER(a.name) LIKE ? OR b.barcode = ? \
             ORDER BY a.name ASC"
        )
            .bind(&like_str)
            .bind(&clean_term)
            .fetch_all(&self.pool)
            .await?;

        Ok(articles)
    }

    pub async fn save(&self, article: &Article, conn: &mut MySqlConnection) -> Result<u64, Error> {
        if let Some(id) = article.id {
            sqlx::query(
                "UPDATE article SET name = ?, price = ?, admission_price1 = ?, admission_price2 = ?, active = ?, category_id = ?, article_image_id = ? WHERE id = ?"
            )
                .bind(&article.name).bind(&article.price).bind(&article.admission_price_1)
                .bind(&article.admission_price_2).bind(article.active).bind(article.category_id)
                .bind(article.article_image_id).bind(id)
                .execute(conn).await?;
            Ok(id)
        } else {
            let res = sqlx::query(
                "INSERT INTO article (name, price, admission_price1, admission_price2, active, category_id, article_image_id) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
                .bind(&article.name).bind(&article.price).bind(&article.admission_price_1)
                .bind(&article.admission_price_2).bind(article.active).bind(article.category_id)
                .bind(article.article_image_id)
                .execute(conn).await?;
            Ok(res.last_insert_id() as u64)
        }
    }

    pub async fn delete(&self, id: u64, conn: &mut MySqlConnection) -> Result<(), Error> {
        sqlx::query("DELETE FROM article WHERE id = ?").bind(id).execute(conn).await?;
        Ok(())
    }

    pub async fn check_name_exists(&self, name: &str) -> Result<bool, Error> {
        let count = sqlx::query_scalar::<_, u64>("SELECT COUNT(1) FROM article WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn check_name_exists_exclude(&self, name: &str, id: u64) -> Result<bool, Error> {
        let count = sqlx::query_scalar::<_, u64>("SELECT COUNT(1) FROM article WHERE name = ? AND id != ?")
            .bind(name)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn get_or_create_category(&self, category_text: &str, conn: &mut MySqlConnection) -> Result<u64, Error> {
        let category_text_trimmed = category_text.trim();

        let res_name = sqlx::query("INSERT IGNORE INTO category_name (name) VALUES (?)")
            .bind(category_text_trimmed)
            .execute(&mut *conn)
            .await?;

        let mut category_name_id = res_name.last_insert_id() as u64;

        if category_name_id == 0 {
            category_name_id = sqlx::query_scalar("SELECT id FROM category_name WHERE name = ?")
                .bind(category_text_trimmed)
                .fetch_one(&mut *conn)
                .await?;
        }

        let res_cat = sqlx::query("INSERT IGNORE INTO category (category_name_id) VALUES (?)")
            .bind(category_name_id)
            .execute(&mut *conn)
            .await?;

        let mut category_id = res_cat.last_insert_id() as u64;

        if category_id == 0 {
            category_id = sqlx::query_scalar("SELECT id FROM category WHERE category_name_id = ?")
                .bind(category_name_id)
                .fetch_one(&mut *conn)
                .await?;
        }

        Ok(category_id)
    }
}
