use crate::error::barcode_error::BarcodeError;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

pub struct SqlxBarcodeRepository {
    pool: MySqlPool,
}

impl SqlxBarcodeRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn verify_barcode_exists(&self, barcode: &str) -> Result<String, BarcodeError> {
        let row = sqlx::query!(
            "SELECT barcode as 'barcode!' FROM article_barcodes WHERE barcode = ?",
            barcode
        )
            .fetch_optional(&self.pool)
            .await
            // Mapiramo tehničku SQLx grešku u naš DatabaseError
            .map_err(|e| BarcodeError::DatabaseError(e.to_string()))?;

        // Mapiramo Option (da li je red pronađen) u našu biznis grešku NotFound
        match row {
            Some(r) => Ok(r.barcode),
            None => Err(BarcodeError::NotFound),
        }
    }

    pub async fn find_by_article_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String)>, Error> {
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
            .fetch_all(&self.pool)
            .await
    }

    pub async fn find_article_id_by_barcode(&self, barcode: &str) -> Result<i64, Error> {
        // 🧠 FIKSIRANO IME METODE: Naziv promenjen iz find_id_by_barcode u find_article_id_by_barcode
        // kako bi se savršeno poklapalo sa pozivom unutar tvog `ArticleService`-a!
        sqlx::query_scalar::<_, i64>("SELECT article_id FROM article_barcodes WHERE barcode = ?")
            .bind(barcode)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn save_barcodes(
        &self,
        article_id: i64,
        barcodes: &Option<Vec<String>>,
        conn: &mut MySqlConnection, // 🧠 Koristimo čistu konekciju umesto generičkih tipova
    ) -> Result<(), Error> {
        if let Some(barcode_list) = barcodes {
            // Čistimo listu od praznih stringova ako ih ima
            let valid_barcodes: Vec<&String> = barcode_list
                .iter()
                .filter(|b| !b.trim().is_empty())
                .collect();

            if !valid_barcodes.is_empty() {
                // Dodajemo IGNORE kako duplikati ne bi srušili transakciju
                let mut qb: QueryBuilder<MySql> =
                    QueryBuilder::new("INSERT IGNORE INTO article_barcodes (article_id, barcode) ");

                qb.push_values(valid_barcodes, |mut b, barcode| {
                    b.push_bind(article_id).push_bind(barcode.trim());
                });

                qb.build().execute(conn).await?;
            }
        }
        Ok(())
    }

    pub async fn delete_barcodes(
        &self,
        article_id: i64,
        conn: &mut MySqlConnection, // 🧠 Koristimo čistu konekciju
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_barcodes WHERE article_id = ?")
            .bind(article_id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
