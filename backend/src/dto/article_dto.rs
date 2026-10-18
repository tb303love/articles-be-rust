use crate::models::{Article, ArticleComposition, ArticleStock};
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ========================================================
// 1. PRIMANJE PODATAKA SA FRONTENDA (Request DTOs)
// ========================================================
#[derive(Debug, Deserialize)]
pub struct ArticleCreateRequest {
    pub name: String,
    pub barcodes: Option<Vec<String>>,
    pub price: Option<BigDecimal>,
    pub admission_price_1: Option<BigDecimal>,
    pub admission_price_2: Option<BigDecimal>,
    pub category_name: Option<String>,
    pub article_image_id: Option<u64>,
    pub active: bool,
    pub components: Option<Vec<ComponentInputDto>>,
    pub initial_stocks: Option<Vec<ArticleStockInputDto>>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleStockInputDto {
    pub quantity: u32,
    pub expiration_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ComponentInputDto {
    pub component_id: u64,
    pub quantity: u32,
}

// src/dto.rs

#[derive(Debug, Serialize, Clone)]
pub struct ArticleStockResponseDto {
    pub id: Option<u64>,
    pub article_id: u64,
    pub quantity: u32,
    pub expiration_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
    pub received_at: chrono::NaiveDateTime, // Vreme kada su zalihe kreirane/primljene
}

// ========================================================
// 2. SLANJE PODATAKA KA FRONTENDU (Response DTOs)
// ========================================================

#[derive(Debug, Serialize)]
pub struct SimpleComponentResponseDto {
    pub id: u64,
    pub name: String,
    pub quantity: u32,
}

// src/dto.rs

#[derive(Debug, Serialize)]
pub struct ArticleResponse {
    pub id: u64,
    pub active: bool,
    pub name: String,
    pub barcodes: Vec<String>,
    pub price: Option<BigDecimal>,
    pub admission_price_1: Option<BigDecimal>,
    pub admission_price_2: Option<BigDecimal>,
    pub category: Option<u64>,
    pub total_stock: u32,
    pub initial_stocks: Option<Vec<ArticleStockResponseDto>>, // 👈 Korišćenje novog Response DTO-a
    pub composition: Option<Vec<SimpleComponentResponseDto>>,
}

// src/dto.rs

impl ArticleResponse {
    pub fn from_entity_with_barcodes(
        article: Article,
        stocks: &[ArticleStock],
        components: &[ArticleComposition],
        barcodes: Vec<String>,
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
            category: article.category_id,
        }
    }
}
