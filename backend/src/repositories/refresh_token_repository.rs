// src/repositories/refresh_token_repository.rs

use sqlx::{Error, FromRow, MySqlPool};
use chrono::NaiveDateTime;

#[derive(FromRow)]
pub struct RefreshTokenRow {
    pub user_id: i64,
    pub expiry_date: NaiveDateTime,
    pub username: String,
    pub role: String,
}

pub struct SqlxRefreshTokenRepository {
    pool: MySqlPool,
}

impl SqlxRefreshTokenRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn save(&self, token: &str, expiry_date: NaiveDateTime, user_id: i64) -> Result<(), Error> {
        // 1. Započinjemo transakciju na bazenu konekcija (BEGIN)
        let mut tx = self.pool.begin().await?;

        // 🧠 FIKSIRANO: Koristimo čist Rust re-borrowing konekcije preko `&mut *tx`
        // što je ujedno i najsigurniji šablon za uzastopne upite unutar transakcije.
        let conn = &mut *tx;

        // 2. Izvršavamo brisanje, ali upit vezujemo za transakciju
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *conn)
            .await?;

        // 3. Izvršavamo unos novog tokena, takođe unutar iste transakcije
        sqlx::query("INSERT INTO refresh_tokens (token, expiry_date, user_id) VALUES (?, ?, ?)")
            .bind(token)
            .bind(expiry_date)
            .bind(user_id)
            .execute(&mut *conn)
            .await?;

        // 4. Ako su oba upita prošla bez greške, trajno ih upisujemo u bazu (COMMIT)
        tx.commit().await?;

        Ok(())
    }

    pub async fn find_with_user(&self, token: &str) -> Result<Option<RefreshTokenRow>, Error> {
        let row = sqlx::query_as::<_, RefreshTokenRow>(
            r#"
            SELECT rt.user_id, rt.expiry_date, u.username, u.role
            FROM refresh_tokens rt
            INNER JOIN users u ON rt.user_id = u.id
            WHERE rt.token = ?
            "#,
        )
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row)
    }

    pub async fn delete(&self, token: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM refresh_tokens WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
