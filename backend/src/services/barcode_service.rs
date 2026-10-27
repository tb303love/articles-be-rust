// src/services/barcode_service.rs

use crate::dto::barcode_dto::BarcodeRequest;
use crate::error::barcode_error::BarcodeError;
// Uvozimo direktno konkretnu strukturu repozitorijuma umesto trait-a
use crate::repositories::SqlxBarcodeRepository;
use std::sync::Arc;

pub struct BarcodeService {
    // 🧠 FIKSIRANO: Menjamo labavu dyn vezu konkretnim tipom
    barcode_repo: Arc<SqlxBarcodeRepository>,
}

impl BarcodeService {
    pub fn new(barcode_repo: Arc<SqlxBarcodeRepository>) -> Self {
        Self { barcode_repo }
    }

    // Servis vraća čist Result, ponaša se kao čista biznis logika
    pub async fn handle_android_scan(&self, barcode: String) -> Result<String, BarcodeError> {
        let trimmed_barcode = barcode.trim();

        if trimmed_barcode.is_empty() {
            return Err(BarcodeError::DatabaseError("Prazan barkod".to_string()));
        }

        // Poziv preko konkretnog tipa prolazi kroz Static Dispatch (munjevito brzo)
        self.barcode_repo
            .verify_barcode_exists(trimmed_barcode)
            .await
    }

    pub async fn handle_stock_input(
        &self,
        request: BarcodeRequest,
    ) -> Result<String, BarcodeError> {
        let trimmed_barcode = request.barcode.trim();
        if trimmed_barcode.is_empty() {
            return Err(BarcodeError::DatabaseError("Prazant barkod".to_string()));
        }

        self.barcode_repo
            .verify_barcode_exists(trimmed_barcode)
            .await
    }
}
