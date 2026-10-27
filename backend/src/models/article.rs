// src/models/article.rs
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use std::collections::HashMap;

use super::article_composition::ArticleComposition;
use super::article_stock::ArticleStock;

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct Article {
    pub id: Option<u64>,
    pub name: String,
    pub active: bool,
    pub price: Option<BigDecimal>,

    // ZADRŽANO: Tačna imena kolona iz tvoje baze podataka!
    #[sqlx(rename = "admission_price1")]
    pub admission_price_1: Option<BigDecimal>,

    #[sqlx(rename = "admission_price2")]
    pub admission_price_2: Option<BigDecimal>,

    pub category_id: Option<u64>,
    pub article_image_id: Option<u64>,
}

impl Article {
    pub fn is_bundle(&self, components: &[ArticleComposition]) -> bool {
        !components.is_empty()
    }

    /// Za običan artikal: zbir svih serija.
    /// Za paket: koliko celih paketa može da se sklopi od trenutnih zaliha komponenti.
    /// `component_stock` mapira ID komponente na njenu ukupnu zalihu.
    pub fn get_total_stock(
        &self,
        stocks: &[ArticleStock],
        components: &[ArticleComposition],
        component_stock: &HashMap<u64, u64>,
    ) -> u64 {
        if self.is_bundle(components) {
            components
                .iter()
                .filter(|c| c.quantity > 0) // Zaštita od deljenja nulom
                .map(|c| {
                    let available = component_stock.get(&c.component_id).copied().unwrap_or(0);
                    available / c.quantity
                })
                .min()
                .unwrap_or(0)
        } else {
            stocks.iter().map(|s| s.quantity).sum()
        }
    }
}
