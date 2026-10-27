// src/repositories/stock_repository.rs

use crate::dto::ArticleStockInputDto;
use crate::models::ArticleStock;
use bigdecimal::BigDecimal;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

pub struct SqlxStockRepository {
    pool: MySqlPool,
}

/// Serija zajedno sa podacima artikla koji ulaze u snapshot stavke otpisa
#[derive(Debug, sqlx::FromRow)]
pub struct StockForWriteOff {
    pub stock_id: u64,
    pub article_id: u64,
    pub article_name: String,
    pub quantity: u64,
    pub batch_number: Option<String>,
    pub nabavna_cena: BigDecimal,
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
        // Otpisane serije ostaju u bazi sa količinom 0, ali se ne prikazuju
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

    /// Zaključava seriju za otpis (nabavna cena je `admission_price1` artikla)
    pub async fn lock_for_write_off(
        &self,
        stock_id: u64,
        conn: &mut MySqlConnection,
    ) -> Result<Option<StockForWriteOff>, Error> {
        sqlx::query_as::<_, StockForWriteOff>(
            "SELECT s.id AS stock_id, a.id AS article_id, a.name AS article_name, s.quantity, \
                    s.batch_number, COALESCE(a.admission_price1, 0) AS nabavna_cena \
             FROM article_stock s JOIN article a ON a.id = s.article_id \
             WHERE s.id = ? FOR UPDATE",
        )
        .bind(stock_id)
        .fetch_optional(conn)
        .await
    }

    /// Otpisana serija ostaje sa količinom 0, da stavka otpisa i dalje može da pokazuje na nju
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
