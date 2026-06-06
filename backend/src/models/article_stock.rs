use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct ArticleStock {
    // 1. Najveći tipovi i Option tipovi (8 bajtova)
    pub id: Option<i64>, // BIGINT UNSIGNED (nullable pre upisa u bazu)
    pub article_id: i64, // Zamena za @ManyToOne Article (nullable = false)
    pub expiration_date: Option<NaiveDate>, // Zamena za LocalDate (nullable)
    pub received_at: NaiveDateTime, // Zamena za LocalDateTime (nullable = false)

    // 2. Dinamičke veličine (String)
    pub batch_number: Option<String>, // nullable string

    // 3. Najmanji tipovi (4 bajta)
    pub quantity: u32, // Smanjeno na u32 jer zalihe ne mogu biti negativne
}

impl ArticleStock {
    /// Konstruktor koji zamenjuje Javin @PrePersist za automatsko postavljanje vremena prijema
    pub fn new(
        article_id: i64,
        quantity: u32,
        expiration_date: Option<NaiveDate>,
        batch_number: Option<String>,
    ) -> Self {
        Self {
            id: None,
            article_id,
            quantity,
            expiration_date,
            batch_number,
            received_at: chrono::Utc::now().naive_utc(), // Trenutno vreme bez vremenske zone
        }
    }
}
