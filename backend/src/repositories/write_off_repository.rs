// src/repositories/write_off_repository.rs

use bigdecimal::BigDecimal;
use sqlx::{Error, MySqlConnection};

pub struct SqlxWriteOffRepository;

impl SqlxWriteOffRepository {
    pub fn new() -> Self {
        Self
    }

    /// Zaglavlje otpisa - vraća ID novog reda u `write_offs`
    pub async fn create(
        &self,
        created_by: Option<u64>,
        napomena: Option<&str>,
        ukupna_vrednost: &BigDecimal,
        conn: &mut MySqlConnection,
    ) -> Result<u64, Error> {
        let res = sqlx::query(
            "INSERT INTO write_offs (created_at, napomena, ukupna_vrednost, created_by) VALUES (?, ?, ?, ?)",
        )
        .bind(chrono::Utc::now().naive_utc())
        .bind(napomena)
        .bind(ukupna_vrednost)
        .bind(created_by)
        .execute(conn)
        .await?;
        Ok(res.last_insert_id())
    }

    pub async fn add_item(
        &self,
        write_off_id: u64,
        article_stock_id: u64,
        quantity: u64,
        price_at_write_off: &BigDecimal,
        conn: &mut MySqlConnection,
    ) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO write_off_items (write_off_id, article_stock_id, quantity, price_at_write_off) VALUES (?, ?, ?, ?)",
        )
        .bind(write_off_id)
        .bind(article_stock_id)
        .bind(quantity)
        .bind(price_at_write_off)
        .execute(conn)
        .await?;
        Ok(())
    }
}
