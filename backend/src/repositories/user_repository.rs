// src/repositories/user_repository.rs

use crate::models::User;
use sqlx::{Error, MySqlConnection, MySqlPool};

pub struct SqlxUserRepository {
    pool: MySqlPool,
}

impl SqlxUserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, Error> {
        log::info!(
            "🔍 DEV LOG: Tražim korisnika u bazi sa imenom: '{}'",
            username
        );

        let result = sqlx::query_as::<_, User>(
            // `role` je nullable u bazi - korisnik bez uloge se tretira kao običan korisnik
            "SELECT id, username, password, COALESCE(role, 'USER') AS role FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await;

        match &result {
            Ok(Some(u)) => log::info!(
                "✅ DEV LOG: Korisnik pronađen! ID: {:?}, Heš iz baze: '{}'",
                u.id,
                u.password
            ),
            Ok(None) => log::warn!(
                "⚠️ DEV LOG: Korisnik '{}' uopšte ne postoji u bazi podataka!",
                username
            ),
            Err(e) => log::error!(
                "❌ DEV LOG: SQLx greška pri mapiranju ili izvršavanju upita: {:?}",
                e
            ),
        }

        result
    }

    pub async fn save(&self, user: &User, conn: &mut MySqlConnection) -> Result<u64, Error> {
        if let Some(id) = user.id {
            sqlx::query("UPDATE users SET username = ?, password = ?, role = ? WHERE id = ?")
                .bind(&user.username)
                .bind(&user.password)
                .bind(&user.role)
                .bind(id)
                .execute(conn)
                .await?;
            Ok(id)
        } else {
            let res = sqlx::query("INSERT INTO users (username, password, role) VALUES (?, ?, ?)")
                .bind(&user.username)
                .bind(&user.password)
                .bind(&user.role)
                .execute(conn)
                .await?;
            Ok(res.last_insert_id() as u64)
        }
    }
}
