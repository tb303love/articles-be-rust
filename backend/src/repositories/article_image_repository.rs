use sqlx::{MySqlConnection, MySqlPool};

pub struct SqlxArticleImageRepository {
    pool: MySqlPool,
}

impl SqlxArticleImageRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Upisuje novu sliku i vraća njen generisani ID
    pub async fn insert(
        &self,
        conn: &mut MySqlConnection,
        file_name: &str,
        file_type: &str,
        original_name: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            INSERT INTO article_images (file_name, file_type, original_name)
            VALUES (?, ?, ?)
            "#,
            file_name,
            file_type,
            original_name
        )
        .execute(conn)
        .await?;

        Ok(result.last_insert_id())
    }

    /// NOVO: Nalazi ime fajla na osnovu ID-ja slike
    pub async fn find_filename_by_id(
        &self,
        id: u64,
        conn: &mut MySqlConnection,
    ) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query!("SELECT file_name FROM article_images WHERE id = ?", id)
            .fetch_optional(conn)
            .await?;

        Ok(row.map(|r| r.file_name))
    }

    /// NOVO: Briše zapis o slici iz baze podataka
    pub async fn delete(&self, id: u64, conn: &mut MySqlConnection) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM article_images WHERE id = ?", id)
            .execute(conn)
            .await?;

        Ok(())
    }
}
