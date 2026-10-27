// src/models/order.rs
use bigdecimal::BigDecimal;
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// U kasi RETAIL znači prodaja sa fiskalnim računom, a WHOLESALE bez njega (vidi `sales.ts`).
/// Cena je u oba slučaja prodajna - tip služi samo za izveštaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Retail,
    Wholesale,
}

impl OrderType {
    pub fn as_db(&self) -> &'static str {
        match self {
            OrderType::Retail => "RETAIL",
            OrderType::Wholesale => "WHOLESALE",
        }
    }

    /// Kolona je nullable ENUM - stari redovi bez tipa se tretiraju kao RETAIL
    pub fn from_db(value: Option<&str>) -> Self {
        match value {
            Some("WHOLESALE") => OrderType::Wholesale,
            _ => OrderType::Retail,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct Order {
    pub id: u64,
    pub created_at: Option<NaiveDateTime>,
    pub total_amount: Option<BigDecimal>,
    #[sqlx(rename = "type")]
    pub order_type: Option<String>,
    pub is_gratis: bool,
}

#[derive(Debug, FromRow)]
pub struct OrderArticle {
    pub id: u64,
    pub order_id: u64,
    pub article_id: Option<u64>,
    pub article_name_snapshot: String,
    pub price_at_sale: BigDecimal,
    pub quantity: i32,
    pub refunded_quantity: i32,
}

/// Stavka porudžbine pre upisa - cene i serija se zamrzavaju u trenutku prodaje
#[derive(Debug)]
pub struct NewOrderArticle {
    pub article_id: u64,
    pub article_name: String,
    pub price_at_sale: BigDecimal,
    pub admission_price_at_sale: BigDecimal,
    pub admission_price2_at_sale: Option<BigDecimal>,
    pub quantity: u64,
    pub batch_number: Option<String>,
    pub expiration_date: Option<NaiveDate>,
}
