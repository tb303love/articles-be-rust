// src/models/article.rs
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

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

    pub fn get_total_stock(
        &self,
        stocks: &[ArticleStock],
        components: &[ArticleComposition],
    ) -> u32 {
        let direct_stock: u32 = stocks.iter().map(|s| s.quantity).sum();

        if self.is_bundle(components) {
            components
                .iter()
                .map(|c| {
                    let component_stock = 10; // Tvoja simulacija iz Jave
                    component_stock / c.quantity
                })
                .min()
                .unwrap_or(0)
        } else {
            direct_stock
        }
    }

    pub fn get_simple_composition<'a>(
        &self,
        components: &'a [ArticleComposition],
    ) -> Option<&'a [ArticleComposition]> {
        if components.is_empty() {
            None
        } else {
            Some(components)
        }
    }
}
