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
    pub article_image_id: Option<i64>,
    pub active: bool,
    pub components: Option<Vec<ComponentInputDto>>,
    pub initial_stocks: Option<Vec<ArticleStockInputDto>>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleStockInputDto {
    pub store_id: i64, // DODATO: Neophodno za MySQL da zna u koji magacin ide roba!
    pub quantity: u32,
    pub expiration_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ComponentInputDto {
    pub component_id: i64,
    pub quantity: u32,
}

// ========================================================
// 2. SLANJE PODATAKA KA FRONTENDU (Response DTOs)
// ========================================================
#[derive(Debug, Serialize)]
pub struct ArticleResponse {
    pub id: i64,
    pub active: bool,
    pub name: String,
    pub barcodes: Vec<String>,
    pub price: Option<BigDecimal>,
    pub admission_price_1: Option<BigDecimal>,
    pub admission_price_2: Option<BigDecimal>,
    pub category: Option<i64>,
    pub total_stock: u32,
    pub composition: Option<Vec<SimpleComponentResponseDto>>,
}

#[derive(Debug, Serialize)]
pub struct SimpleComponentResponseDto {
    pub id: i64,
    pub name: String,
    pub quantity: u32,
}

impl ArticleResponse {
    // Genijalna metoda koja transformiše bazične modele u čist frontend JSON
    pub fn from_entity_with_barcodes(
        article: Article,
        stocks: &[ArticleStock],
        components: &[ArticleComposition],
        barcodes: Vec<String>,
    ) -> Self {
        // Pozivamo tvoje biznis metode iz modela za računanje stanja i sastava
        let total_stock = article.get_total_stock(stocks, components);

        // Unutar dtos.rs fajla, u metodi from_entity_with_barcodes:

        let composition = article.get_simple_composition(components).map(|comps| {
            comps
                .iter() // Koristimo .iter() jer radimo sa referencom niza
                .map(|c| SimpleComponentResponseDto {
                    id: c.component_id,  // FIKSIRANO: Koristimo component_id koji je i64
                    name: String::new(), // Naziv ostaje prazan ili se uparuje naknadno
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
            composition,
            category: article.category_id,
        }
    }
}
