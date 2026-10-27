// src/repositories/order_repository.rs

use crate::models::{NewOrderArticle, Order, OrderArticle, OrderType};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use sqlx::{Error, MySql, MySqlConnection, MySqlPool, QueryBuilder};

pub struct SqlxOrderRepository {
    pool: MySqlPool,
}

impl SqlxOrderRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn insert_order(
        &self,
        total_amount: &BigDecimal,
        order_type: OrderType,
        is_gratis: bool,
        conn: &mut MySqlConnection,
    ) -> Result<u64, Error> {
        let res = sqlx::query(
            "INSERT INTO orders (created_at, total_amount, type, is_gratis) VALUES (?, ?, ?, ?)",
        )
        .bind(chrono::Utc::now().naive_utc())
        .bind(total_amount)
        .bind(order_type.as_db())
        .bind(is_gratis)
        .execute(conn)
        .await?;
        Ok(res.last_insert_id())
    }

    pub async fn insert_items(
        &self,
        order_id: u64,
        items: &[NewOrderArticle],
        conn: &mut MySqlConnection,
    ) -> Result<(), Error> {
        if items.is_empty() {
            return Ok(());
        }
        let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
            "INSERT INTO order_articles (order_id, article_id, article_name_snapshot, price_at_sale, \
             admission_price_at_sale, admission_price2_at_sale, quantity, refunded_quantity, batch_number, expiration_date) ",
        );
        qb.push_values(items, |mut b, item| {
            b.push_bind(order_id)
                .push_bind(item.article_id)
                .push_bind(&item.article_name)
                .push_bind(&item.price_at_sale)
                .push_bind(&item.admission_price_at_sale)
                .push_bind(&item.admission_price2_at_sale)
                .push_bind(item.quantity)
                .push_bind(0)
                .push_bind(&item.batch_number)
                .push_bind(item.expiration_date);
        });
        qb.build().execute(conn).await?;
        Ok(())
    }

    pub async fn find_by_id(&self, id: u64) -> Result<Option<Order>, Error> {
        sqlx::query_as::<_, Order>(
            "SELECT id, created_at, total_amount, type, is_gratis FROM orders WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Porudžbine u intervalu [od, do), najnovije prve
    pub async fn find_between(
        &self,
        od: NaiveDateTime,
        do_: NaiveDateTime,
    ) -> Result<Vec<Order>, Error> {
        sqlx::query_as::<_, Order>(
            "SELECT id, created_at, total_amount, type, is_gratis FROM orders \
             WHERE created_at >= ? AND created_at < ? ORDER BY created_at DESC, id DESC",
        )
        .bind(od)
        .bind(do_)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn find_items_by_order_ids(&self, ids: &[u64]) -> Result<Vec<OrderArticle>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, order_id, article_id, article_name_snapshot, price_at_sale, quantity, refunded_quantity \
             FROM order_articles WHERE order_id IN (",
        );
        let mut sep = builder.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY id");

        builder
            .build_query_as::<OrderArticle>()
            .fetch_all(&self.pool)
            .await
    }

    pub async fn update_type(&self, id: u64, order_type: OrderType) -> Result<u64, Error> {
        let res = sqlx::query("UPDATE orders SET type = ? WHERE id = ?")
            .bind(order_type.as_db())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    /// Zbir iznosa po tipu porudžbine u intervalu [od, do)
    pub async fn sum_by_type_between(
        &self,
        od: NaiveDateTime,
        do_: NaiveDateTime,
    ) -> Result<Vec<(Option<String>, BigDecimal)>, Error> {
        sqlx::query_as::<_, (Option<String>, BigDecimal)>(
            "SELECT type, COALESCE(SUM(total_amount), 0) FROM orders \
             WHERE created_at >= ? AND created_at < ? GROUP BY type",
        )
        .bind(od)
        .bind(do_)
        .fetch_all(&self.pool)
        .await
    }
}
