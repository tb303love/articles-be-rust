#[derive(Debug)]
pub enum BarcodeError {
    NotFound,              // Biznis greška: Artikal ne postoji u bazi
    DatabaseError(String), // Tehnička greška: Pukla veza, pogrešan SQL itd.
}
