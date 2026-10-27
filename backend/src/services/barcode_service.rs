// src/services/barcode_service.rs

use crate::dto::barcode_dto::BarcodeRequest;
use crate::error::ApiError;
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
    pub async fn handle_android_scan(&self, barcode: String) -> Result<String, ApiError> {
        self.verify(&barcode).await
    }

    pub async fn handle_stock_input(&self, request: BarcodeRequest) -> Result<String, ApiError> {
        self.verify(&request.barcode).await
    }

    async fn verify(&self, barcode: &str) -> Result<String, ApiError> {
        let trimmed_barcode = barcode.trim();
        if trimmed_barcode.is_empty() {
            return Err(ApiError::BadRequest("Barkod je prazan.".into()));
        }

        // Poziv preko konkretnog tipa prolazi kroz Static Dispatch (munjevito brzo)
        self.barcode_repo
            .verify_barcode_exists(trimmed_barcode)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Artikal sa barkodom {} ne postoji.", trimmed_barcode))
            })
    }
}
