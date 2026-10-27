use crate::models::{Article, ArticleComposition, ArticleImage, ArticleStock};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize)]
pub struct SimpleComponentResponseDto {
    pub id: u64,
    pub name: String,
    pub quantity: u64,
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
    pub price: Option<BigDecimal>,
    pub admission_price_1: Option<BigDecimal>,
    pub admission_price_2: Option<BigDecimal>,
    pub category: Option<ArticleCategory>,
    pub total_stock: u64,
    pub image: Option<ArticleImage>,
    pub initial_stocks: Option<Vec<ArticleStockResponseDto>>, // 👈 Korišćenje novog Response DTO-a
    pub composition: Option<Vec<SimpleComponentResponseDto>>,
}

impl ArticleResponse {
    pub fn from_entity_with_barcodes(
        article: Article,
        stocks: &[ArticleStock],
        components: &[ArticleComposition],
        barcodes: Vec<String>,
        category: Option<ArticleCategory>,
        image: Option<ArticleImage>,
    ) -> Self {
        // Računanje ukupnog stanja zaliha na osnovu biznis pravila iz modela
        let total_stock = article.get_total_stock(stocks, components);

        // 🧠 MAPIRANJE: Sada prepisujemo stvarna polja koja postoje u tvom ArticleStock modelu!
        let initial_stocks_dto = if stocks.is_empty() {
            None
        } else {
            Some(
                stocks
                    .iter()
                    .map(|s| ArticleStockResponseDto {
                        id: s.id,
                        article_id: s.article_id,
                        quantity: s.quantity,
                        expiration_date: s.expiration_date,
                        batch_number: s.batch_number.clone(),
                        received_at: s.received_at, // Mapiramo NaiveDateTime
                    })
                    .collect(),
            )
        };

        let composition_dto = article.get_simple_composition(components).map(|comps| {
            comps
                .iter()
                .map(|c| SimpleComponentResponseDto {
                    id: c.component_id,
                    name: String::new(),
                    quantity: c.quantity,
                })
                .collect()
        });

        Self {
            id: article.id.unwrap_or(0),
            active: article.active,
            name: article.name,
            barcodes,
            price: article.price,
            admission_price_1: article.admission_price_1,
            admission_price_2: article.admission_price_2,
            total_stock,
            initial_stocks: initial_stocks_dto, // Smeštamo ispravan, mapirani niz
            composition: composition_dto,
            category,
            image,
        }
    }
}
