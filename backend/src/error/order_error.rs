#[derive(Debug)]
pub enum OrderError {
    NotFound(String),          // Porudžbina ili artikal ne postoji
    BadRequest(String),        // Neispravan zahtev (prazna porudžbina, količina 0...)
    InsufficientStock(String), // Biznis greška: nema dovoljno zalihe - frontend je prikazuje kao konflikt (409)
    Database(sqlx::Error),     // Tehnička greška
}

impl From<sqlx::Error> for OrderError {
    fn from(e: sqlx::Error) -> Self {
        OrderError::Database(e)
    }
}
