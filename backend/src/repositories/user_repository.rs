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
        let row = sqlx::query_as::<_, User>(
            "SELECT id, username, password, role FROM users WHERE username = ?",
        )
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn save(&self, user: &User, conn: &mut MySqlConnection) -> Result<i64, Error> {
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
            Ok(res.last_insert_id() as i64)
        }
    }
}
