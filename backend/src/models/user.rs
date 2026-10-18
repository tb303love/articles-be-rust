// src/models/user.rs
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Option<u64>,
    pub username: String,
    pub password: String, // Hešovana lozinka (bcrypt)
    pub role: String,     // Npr. "ADMIN", "USER"
}
