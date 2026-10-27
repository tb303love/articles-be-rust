use serde::Serialize;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ArticleImage {
    pub id: u64,
    pub file_name: String,
    pub file_type: String,
    pub original_name: String,
}

// Struktura u koju pakujemo sirove bajtove i meta-podatke tokom multipart obrade
pub struct ImageUploadData {
    pub bytes: Vec<u8>,
    pub original_name: String,
    pub content_type: String,
}
