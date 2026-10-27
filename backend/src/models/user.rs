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

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum UserRole {
    Admin,
    User,
}

impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.to_uppercase().as_str() {
            "ADMIN" => UserRole::Admin,
            _ => UserRole::User,
        }
    }
}
