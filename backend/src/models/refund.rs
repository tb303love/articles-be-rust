// src/models/refund.rs
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct Refund {
    pub id: u64,
    pub created_at: Option<NaiveDateTime>,
    pub total_amount: Option<BigDecimal>,
    pub original_order_id: Option<u64>,
}

/// Stavka refundacije sa imenom artikla (refund_items nema snapshot, ime dolazi iz `article`)
#[derive(Debug, FromRow)]
pub struct RefundItem {
    pub id: u64,
    pub refund_id: Option<u64>,
    pub article_id: Option<u64>,
    pub article_name: Option<String>,
    pub quantity: Option<i32>,
}
