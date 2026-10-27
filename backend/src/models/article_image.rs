use serde::Serialize;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ArticleImage {
    pub id: u64,
    pub file_name: String,
    pub file_type: Option<String>,     // Nullable u bazi (stari redovi)
    pub original_name: Option<String>, // Nullable u bazi (stari redovi)
}

// Struktura u koju pakujemo sirove bajtove i meta-podatke tokom multipart obrade
pub struct ImageUploadData {
    pub bytes: Vec<u8>,
    pub original_name: String,
    pub content_type: String,
}
