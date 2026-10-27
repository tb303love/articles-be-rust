// src/dto/order_dto.rs
use crate::models::OrderType;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderRequest {
    pub items: Vec<OrderItemRequest>,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub is_gratis: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemRequest {
    pub article_id: u64,
    pub quantity: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderTypeQuery {
    pub new_type: OrderType,
}

// --- ODGOVORI (oblik iz `order.model.ts` i `refund.model.ts`) ---
// Vreme ide kao UTC sa "Z" na kraju, pa ga browser sam prikazuje u lokalnom vremenu

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub id: u64,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub created_at: Option<DateTime<Utc>>,
    #[serde(with = "bigdecimal::serde::json_num")]
    pub total_amount: BigDecimal,
    pub is_gratis: bool,
    pub items: Vec<OrderItemResponse>,
    pub refunds: Vec<RefundResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemResponse {
    pub id: u64,
    pub article_id: Option<u64>, // NULL ako je artikal u međuvremenu obrisan
    pub article_name: String,
    #[serde(with = "bigdecimal::serde::json_num")]
    pub price_at_sale: BigDecimal,
    pub quantity: i32,
    pub refunded_quantity: i32,
    pub available_to_refund: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    pub id: u64,
    pub original_order_id: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
    #[serde(with = "bigdecimal::serde::json_num")]
    pub total_amount: BigDecimal,
    pub items: Vec<RefundItemResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundItemResponse {
    pub id: u64,
    pub article_id: Option<u64>,
    pub article_name: String,
    pub quantity: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevenueSummary {
    #[serde(with = "bigdecimal::serde::json_num")]
    pub gross_retail: BigDecimal,
    #[serde(with = "bigdecimal::serde::json_num")]
    pub gross_wholesale: BigDecimal,
    #[serde(with = "bigdecimal::serde::json_num")]
    pub total_refunds: BigDecimal,
    #[serde(with = "bigdecimal::serde::json_num")]
    pub net_revenue: BigDecimal,
}
