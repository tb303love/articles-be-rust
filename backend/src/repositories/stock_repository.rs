// src/repositories/stock_repository.rs

use crate::dto::ArticleStockInputDto;
use crate::models::ArticleStock;
use bigdecimal::BigDecimal;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

pub struct SqlxStockRepository {
    pool: MySqlPool,
}

impl SqlxStockRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    // 🧠 FIKSIRANO: Selektujemo tačne kolone koje tvoj Rust model ArticleStock poseduje!
    pub async fn find_stocks_by_article_ids(
        &self,
        ids: &[u64],
    ) -> Result<Vec<ArticleStock>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // Otpisane serije ostaju u bazi sa količinom 0 (na njih pokazuje `write_off_items`), ali se ne prikazuju
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, article_id, quantity, expiration_date, batch_number, received_at FROM article_stock WHERE quantity > 0 AND article_id IN (",
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

    // Zadržavamo i ovu metodu ako je tvoj servis poziva pod drugim imenom, ali sa ispravnim kolonama
    pub async fn find_by_article_ids(&self, ids: &[u64]) -> Result<Vec<ArticleStock>, Error> {
        self.find_stocks_by_article_ids(ids).await
    }

    // 🧠 NOVO: Dodata metoda za tvoj modal na frontendu (kopirana Spring Boot logika)
    pub async fn find_affected_bundles(&self, stock_id: u64) -> Result<Vec<String>, Error> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT bundle_article.name \
             FROM article_composition ac \
             JOIN article bundle_article ON ac.bundle_id = bundle_article.id \
             WHERE ac.component_id = (SELECT s.article_id FROM article_stock s WHERE s.id = ?)",
        )
        .bind(stock_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // 🧠 FIKSIRANO: Upisujemo podatke prateći strukturu tabele i automatski @PrePersist tajming
    pub async fn save_stocks(
        &self,
        article_id: u64,
        stocks: &[ArticleStockInputDto],
        conn: &mut MySqlConnection, // Primamo čistu konekciju unutar transakcije
    ) -> Result<(), Error> {
        if stocks.is_empty() {
            return Ok(());
        }

        let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT INTO article_stock (article_id, quantity, expiration_date, batch_number, received_at) ",
        );

        // Menja Javin @PrePersist na nivou baze, sve zalihe u ovom ciklusu dobijaju isti timestamp
        let trenutno_vreme = chrono::Utc::now().naive_utc();

        qb.push_values(stocks, |mut b, stock| {
            b.push_bind(article_id)
                .push_bind(stock.quantity)
                .push_bind(stock.expiration_date) // Sačuvaće NaiveDate ili NULL
                .push_bind(&stock.batch_number) // Sačuvaće String ili NULL
                .push_bind(trenutno_vreme);
        });

        qb.build().execute(conn).await?;
        Ok(())
    }

    /// Zaključava seriju za otpis i vraća (količina, broj serije, nabavna cena artikla)
    pub async fn lock_for_write_off(
        &self,
        stock_id: u64,
        conn: &mut MySqlConnection,
    ) -> Result<Option<(u64, Option<String>, BigDecimal)>, Error> {
        sqlx::query_as::<_, (u64, Option<String>, BigDecimal)>(
            "SELECT s.quantity, s.batch_number, COALESCE(a.admission_price1, 0) \
             FROM article_stock s JOIN article a ON a.id = s.article_id \
             WHERE s.id = ? FOR UPDATE",
        )
        .bind(stock_id)
        .fetch_optional(conn)
        .await
    }

    /// Serija se ne briše - `write_off_items` ima ON DELETE CASCADE i obrisao bi se i sam otpis
    pub async fn clear_quantity(
        &self,
        stock_id: u64,
        conn: &mut MySqlConnection,
    ) -> Result<(), Error> {
        sqlx::query("UPDATE article_stock SET quantity = 0 WHERE id = ?")
            .bind(stock_id)
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete_stocks(
        &self,
        article_id: u64,
        conn: &mut MySqlConnection, // Primamo čistu konekciju unutar transakcije
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM article_stock WHERE article_id = ?")
            .bind(article_id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
