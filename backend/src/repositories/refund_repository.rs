// src/repositories/refund_repository.rs

use crate::models::{Refund, RefundItem};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use sqlx::{Error, MySql, MySqlPool, QueryBuilder};

pub struct SqlxRefundRepository {
    pool: MySqlPool,
}

impl SqlxRefundRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_order_ids(&self, ids: &[u64]) -> Result<Vec<Refund>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, created_at, total_amount, original_order_id FROM refunds WHERE original_order_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY id");

        builder
            .build_query_as::<Refund>()
            .fetch_all(&self.pool)
            .await
    }

    pub async fn find_items_by_refund_ids(&self, ids: &[u64]) -> Result<Vec<RefundItem>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT ri.id, ri.refund_id, ri.article_id, a.name AS article_name, ri.quantity \
             FROM refund_items ri LEFT JOIN article a ON a.id = ri.article_id \
             WHERE ri.refund_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY ri.id");

        builder
            .build_query_as::<RefundItem>()
            .fetch_all(&self.pool)
            .await
    }

    /// Ukupno vraćen novac u intervalu [od, do) - po datumu refundacije, ne originalne porudžbine
    pub async fn sum_between(
        &self,
        od: NaiveDateTime,
        do_: NaiveDateTime,
    ) -> Result<BigDecimal, Error> {
        sqlx::query_scalar::<_, BigDecimal>(
            "SELECT COALESCE(SUM(total_amount), 0) FROM refunds WHERE created_at >= ? AND created_at < ?",
        )
        .bind(od)
        .bind(do_)
        .fetch_one(&self.pool)
        .await
    }
}
