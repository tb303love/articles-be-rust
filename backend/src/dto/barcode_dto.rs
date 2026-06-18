use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BarcodeRequest {
    pub barcode: String,
}
