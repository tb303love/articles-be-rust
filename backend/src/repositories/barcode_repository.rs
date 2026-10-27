use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

pub struct SqlxBarcodeRepository {
    pool: MySqlPool,
}

impl SqlxBarcodeRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn verify_barcode_exists(&self, barcode: &str) -> Result<Option<String>, Error> {
        let row = sqlx::query!(
            "SELECT barcode as 'barcode!' FROM article_barcodes WHERE barcode = ?",
            barcode
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.barcode))
    }

    pub async fn find_by_article_ids(&self, ids: &[u64]) -> Result<Vec<(u64, String)>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            // `barcode` je nullable u bazi - takve redove preskačemo
            "SELECT article_id, barcode FROM article_barcodes WHERE barcode IS NOT NULL AND article_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");

        builder
            .build_query_as::<(u64, String)>()
            .fetch_all(&self.pool)
            .await
    }

    pub async fn find_article_id_by_barcode(&self, barcode: &str) -> Result<u64, Error> {
        // 🧠 FIKSIRANO IME METODE: Naziv promenjen iz find_id_by_barcode u find_article_id_by_barcode
        // kako bi se savršeno poklapalo sa pozivom unutar tvog `ArticleService`-a!
        sqlx::query_scalar::<_, u64>("SELECT article_id FROM article_barcodes WHERE barcode = ?")
            .bind(barcode)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn save_barcodes(
        &self,
        article_id: u64,
        barcodes: &Option<Vec<String>>,
        conn: &mut MySqlConnection, // 🧠 Koristimo čistu konekciju umesto generičkih tipova
    ) -> Result<(), Error> {
        if let Some(barcode_list) = barcodes {
            // Čistimo listu od praznih stringova i ponovljenih barkodova iz istog zahteva
            let mut valid_barcodes: Vec<&str> = barcode_list
                .iter()
                .map(|b| b.trim())
                .filter(|b| !b.is_empty())
                .collect();
            valid_barcodes.sort_unstable();
            valid_barcodes.dedup();

            if !valid_barcodes.is_empty() {
                // Bez IGNORE: barkod koji već pripada drugom artiklu mora da obori upis (409),
                // a ne da se tiho preskoči pa artikal ostane bez barkoda
                let mut qb: QueryBuilder<MySql> =
                    QueryBuilder::new("INSERT INTO article_barcodes (article_id, barcode) ");

                qb.push_values(valid_barcodes, |mut b, barcode| {
                    b.push_bind(article_id).push_bind(barcode);
                });

                qb.build().execute(conn).await?;
            }
        }
        Ok(())
    }

    pub async fn delete_barcodes(
        &self,
        article_id: u64,
        conn: &mut MySqlConnection, // 🧠 Koristimo čistu konekciju
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_barcodes WHERE article_id = ?")
            .bind(article_id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
