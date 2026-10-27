// src/repositories/write_off_repository.rs

use super::stock_repository::StockForWriteOff;
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

    /// Stavka čuva snapshot imena i serije - ostaje čitljiva i kada se serija ili artikal obrišu
    pub async fn add_item(
        &self,
        write_off_id: u64,
        stock: &StockForWriteOff,
        conn: &mut MySqlConnection,
    ) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO write_off_items \
             (write_off_id, article_stock_id, article_id, article_name_snapshot, batch_number_snapshot, quantity, price_at_write_off) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(write_off_id)
        .bind(stock.stock_id)
        .bind(stock.article_id)
        .bind(&stock.article_name)
        .bind(&stock.batch_number)
        .bind(stock.quantity)
        .bind(&stock.nabavna_cena)
        .execute(conn)
        .await?;
        Ok(())
    }
}
