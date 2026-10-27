use crate::models::{Article, ArticleComposition, ArticleImage, ArticleStock};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCreateRequest {
    pub name: String,
    pub active: bool,
    pub price: Option<BigDecimal>,
    pub admission_price_1: Option<BigDecimal>,
    pub admission_price_2: Option<BigDecimal>,
    // Relacioni podaci za povezane tabele
    pub barcodes: Option<Vec<String>>,
    pub category_name: Option<String>,
    pub components: Option<Vec<ComponentInputDto>>,
    pub initial_stocks: Option<Vec<ArticleStockInputDto>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleUpdateRequest {
    pub name: String,
    pub active: bool,
    pub price: Option<BigDecimal>,
    pub admission_price_1: Option<BigDecimal>,
    pub admission_price_2: Option<BigDecimal>,
    // Relacioni podaci za povezane tabele
    pub category_name: Option<String>,
    pub barcodes: Option<Vec<String>>,
    pub components: Option<Vec<ComponentInputDto>>,
    pub initial_stocks: Option<Vec<ArticleStockInputDto>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleStockInputDto {
    pub quantity: u64,
    pub expiration_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentInputDto {
    pub component_id: u64,
    pub quantity: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleStockResponseDto {
    pub id: Option<u64>,
    pub article_id: u64,
    pub quantity: u64,
    pub expiration_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
    pub received_at: chrono::NaiveDateTime,
}

impl From<ArticleStock> for ArticleStockResponseDto {
    fn from(s: ArticleStock) -> Self {
        Self {
            id: s.id,
            article_id: s.article_id,
            quantity: s.quantity,
            expiration_date: s.expiration_date,
            batch_number: s.batch_number,
            received_at: s.received_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleComponentResponseDto {
    pub article_id: u64, // Frontend očekuje `articleId`
    pub name: String,
    pub quantity: u64,
}

/// Javni oblik slike - samo ono što frontendu treba za prikaz i edit mod
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleImageResponse {
    pub id: u64,
    pub file_name: String,
    pub file_type: String,
    pub original_name: String,
}

impl From<ArticleImage> for ArticleImageResponse {
    fn from(img: ArticleImage) -> Self {
        // Frontend očekuje stringove - za stare redove bez meta-podataka dajemo razumne zamene
        Self {
            id: img.id,
            original_name: img.original_name.unwrap_or_else(|| img.file_name.clone()),
            file_type: img.file_type.unwrap_or_default(),
            file_name: img.file_name,
        }
    }
}
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCategoryName {
    pub name: String,
    pub id: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCategory {
    pub id: u64,
    pub category_name: ArticleCategoryName,
}

#[derive(serde::Serialize)]
pub struct CategoryNameResponse {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleResponse {
    pub id: u64,
    pub active: bool,
    pub name: String,
    pub barcodes: Vec<String>,
    // Cene idu kao JSON brojevi (120.5), ne kao stringovi ("120.50") - frontend ih vodi kao `number`
    #[serde(with = "bigdecimal::serde::json_num_option")]
    pub price: Option<BigDecimal>,
    #[serde(with = "bigdecimal::serde::json_num_option")]
    pub admission_price_1: Option<BigDecimal>,
    #[serde(with = "bigdecimal::serde::json_num_option")]
    pub admission_price_2: Option<BigDecimal>,
    pub category: Option<ArticleCategory>,
    pub total_stock: u64,
    pub image: Option<ArticleImageResponse>,
    pub stocks: Option<Vec<ArticleStockResponseDto>>, // Frontend čita `stocks`
    pub composition: Option<Vec<SimpleComponentResponseDto>>,
}

/// Podaci o komponentama paketa, deljeni za ceo spisak artikala
#[derive(Debug, Default)]
pub struct ComponentLookup {
    pub names: HashMap<u64, String>,
    pub stock: HashMap<u64, u64>, // Ukupna zaliha po ID-ju komponente
}

/// Svi povezani podaci za jedan artikal, već učitani iz baze
#[derive(Debug, Default)]
pub struct ArticleRelations {
    pub barcodes: Vec<String>,
    pub stocks: Vec<ArticleStock>,
    pub components: Vec<ArticleComposition>,
    pub category: Option<ArticleCategory>,
    pub image: Option<ArticleImage>,
}

impl ArticleResponse {
    pub fn from_parts(article: Article, rel: ArticleRelations, lookup: &ComponentLookup) -> Self {
        // Računanje ukupnog stanja zaliha mora pre nego što `stocks` bude potrošen ispod
        let total_stock = article.get_total_stock(&rel.stocks, &rel.components, &lookup.stock);

        let stocks = (!rel.stocks.is_empty())
            .then(|| rel.stocks.into_iter().map(Into::into).collect());

        let composition = (!rel.components.is_empty()).then(|| {
            rel.components
                .into_iter()
                .map(|c| SimpleComponentResponseDto {
                    article_id: c.component_id,
                    name: lookup
                        .names
                        .get(&c.component_id)
                        .cloned()
                        .unwrap_or_default(),
                    quantity: c.quantity,
                })
                .collect()
        });

        Self {
            id: article.id.expect("artikal učitan iz baze uvek ima id"),
            active: article.active,
            name: article.name,
            barcodes: rel.barcodes,
            price: article.price,
            admission_price_1: article.admission_price_1,
            admission_price_2: article.admission_price_2,
            total_stock,
            stocks,
            composition,
            category: rel.category,
            image: rel.image.map(Into::into),
        }
    }
}
