use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCategoryName {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCategory {
    pub id: u64,
    pub category_name: ArticleCategoryName, // 👈 Dodato pub!
}
