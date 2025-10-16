use crate::models::{Order, OrderError, OrderId, OrderSide, OrderStatus, OrderType, UserId};
use chrono::{DateTime, Utc};
use color_eyre::Result;
use database_adapter::db::DbError;
use sqlx::{PgPool, Row};
use tracing::debug;
use uuid::Uuid;

pub struct OrderRepository {
    pool: PgPool,
}

impl OrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_order(&self, order: Order) -> Result<(OrderId, Order), OrderError> {
        let order_id = Uuid::new_v4();
        debug!("Creating order with ID: {}", order_id);

        // Extract order_type fields
        let (order_type_variant, order_type_price) = match &order.order_type {
            OrderType::Market => ("Market", None),
            OrderType::Limit { price } => ("Limit", Some(*price)),
        };

        // Extract status fields
        let (status_variant, status_filled_quantity, status_date) = match &order.status {
            OrderStatus::Pending => ("Pending", None, None),
            OrderStatus::PartiallyFilled { filled_quantity } => {
                ("PartiallyFilled", Some(*filled_quantity as i64), None)
            }
            OrderStatus::Filled { date } => ("Filled", Some(order.quantity as i64), Some(*date)),
            OrderStatus::Rejected { date } => ("Rejected", None, Some(*date)),
            OrderStatus::Cancelled { date } => ("Cancelled", None, Some(*date)),
        };

        let order_side_str = match order.order_side {
            OrderSide::Buy => "Buy",
            OrderSide::Sell => "Sell",
        };

        sqlx::query(
            r#"
            INSERT INTO orders (
                id, client_id, date, symbol, quantity, order_side,
                order_type_variant, order_type_price,
                status_variant, status_filled_quantity, status_date,
                filled_price
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(order_id)
        .bind(order.client_id)
        .bind(order.date)
        .bind(&order.symbol)
        .bind(order.quantity as i64)
        .bind(order_side_str)
        .bind(order_type_variant)
        .bind(order_type_price)
        .bind(status_variant)
        .bind(status_filled_quantity)
        .bind(status_date)
        .bind(order.filled_price)
        .execute(&self.pool)
        .await
        .map_err(|e| OrderError::Repository(DbError::SqlxError(e)))?;

        Ok((order_id, order))
    }

    pub async fn get_orders_by_user(
        &self,
        user_id: &UserId,
        page: u32,
        limit: u32,
    ) -> Result<Vec<(OrderId, Order)>, OrderError> {
        debug!(
            "Getting orders for user: {} (page: {}, limit: {})",
            user_id, page, limit
        );

        let offset = ((page - 1) * limit) as i64;

        let rows = sqlx::query(
            r#"
            SELECT 
                id, client_id, date, symbol, quantity, order_side,
                order_type_variant, order_type_price,
                status_variant, status_filled_quantity, status_date,
                filled_price
            FROM orders
            WHERE client_id = $1
            ORDER BY date DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OrderError::Repository(DbError::SqlxError(e)))?;

        let orders = rows
            .into_iter()
            .map(|row| {
                let id: Uuid = row.get("id");
                let client_id: Uuid = row.get("client_id");
                let date: DateTime<Utc> = row.get("date");
                let symbol: String = row.get("symbol");
                let quantity: i64 = row.get("quantity");
                let order_side_str: String = row.get("order_side");
                let order_type_variant: String = row.get("order_type_variant");
                let order_type_price: Option<f64> = row.get("order_type_price");
                let status_variant: String = row.get("status_variant");
                let status_filled_quantity: Option<i64> = row.get("status_filled_quantity");
                let status_date: Option<DateTime<Utc>> = row.get("status_date");
                let filled_price: Option<f64> = row.get("filled_price");

                let order_side = match order_side_str.as_str() {
                    "Buy" => OrderSide::Buy,
                    "Sell" => OrderSide::Sell,
                    _ => OrderSide::Buy,
                };

                let order_type = match order_type_variant.as_str() {
                    "Market" => OrderType::Market,
                    "Limit" => OrderType::Limit {
                        price: order_type_price.unwrap_or(0.0),
                    },
                    _ => OrderType::Market,
                };

                let status = match status_variant.as_str() {
                    "Pending" => OrderStatus::Pending,
                    "PartiallyFilled" => OrderStatus::PartiallyFilled {
                        filled_quantity: status_filled_quantity.unwrap_or(0) as u64,
                    },
                    "Filled" => OrderStatus::Filled {
                        date: status_date.unwrap_or_else(Utc::now),
                    },
                    "Rejected" => OrderStatus::Rejected {
                        date: status_date.unwrap_or_else(Utc::now),
                    },
                    "Cancelled" => OrderStatus::Cancelled {
                        date: status_date.unwrap_or_else(Utc::now),
                    },
                    _ => OrderStatus::Pending,
                };

                let order = Order {
                    client_id,
                    date,
                    symbol,
                    quantity: quantity as u64,
                    order_side,
                    order_type,
                    status,
                    filled_price,
                };

                (id, order)
            })
            .collect();

        Ok(orders)
    }

    pub async fn count_orders_by_user(&self, user_id: &UserId) -> Result<i64, OrderError> {
        debug!("Counting orders for user: {}", user_id);

        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM orders
            WHERE client_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| OrderError::Repository(DbError::SqlxError(e)))?;

        let count: i64 = row.get("count");
        Ok(count)
    }

    pub async fn get_order_by_id(&self, order_id: &OrderId) -> Result<Option<Order>, OrderError> {
        debug!("Getting order by ID: {}", order_id);

        let row = sqlx::query(
            r#"
            SELECT 
                id, client_id, date, symbol, quantity, order_side,
                order_type_variant, order_type_price,
                status_variant, status_filled_quantity, status_date,
                filled_price
            FROM orders
            WHERE id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OrderError::Repository(DbError::SqlxError(e)))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let client_id: Uuid = row.get("client_id");
                let date: DateTime<Utc> = row.get("date");
                let symbol: String = row.get("symbol");
                let quantity: i64 = row.get("quantity");
                let order_side_str: String = row.get("order_side");
                let order_type_variant: String = row.get("order_type_variant");
                let order_type_price: Option<f64> = row.get("order_type_price");
                let status_variant: String = row.get("status_variant");
                let status_filled_quantity: Option<i64> = row.get("status_filled_quantity");
                let status_date: Option<DateTime<Utc>> = row.get("status_date");
                let filled_price: Option<f64> = row.get("filled_price");

                let order_side = match order_side_str.as_str() {
                    "Buy" => OrderSide::Buy,
                    "Sell" => OrderSide::Sell,
                    _ => OrderSide::Buy,
                };

                let order_type = match order_type_variant.as_str() {
                    "Market" => OrderType::Market,
                    "Limit" => OrderType::Limit {
                        price: order_type_price.unwrap_or(0.0),
                    },
                    _ => OrderType::Market,
                };

                let status = match status_variant.as_str() {
                    "Pending" => OrderStatus::Pending,
                    "PartiallyFilled" => OrderStatus::PartiallyFilled {
                        filled_quantity: status_filled_quantity.unwrap_or(0) as u64,
                    },
                    "Filled" => OrderStatus::Filled {
                        date: status_date.unwrap_or_else(Utc::now),
                    },
                    "Rejected" => OrderStatus::Rejected {
                        date: status_date.unwrap_or_else(Utc::now),
                    },
                    "Cancelled" => OrderStatus::Cancelled {
                        date: status_date.unwrap_or_else(Utc::now),
                    },
                    _ => OrderStatus::Pending,
                };

                let order = Order {
                    client_id,
                    date,
                    symbol,
                    quantity: quantity as u64,
                    order_side,
                    order_type,
                    status,
                    filled_price,
                };

                Ok(Some(order))
            }
        }
    }

    pub async fn update_order(&self, order_id: OrderId, order: Order) -> Result<(), OrderError> {
        debug!("Updating order {}", order_id);

        // Extract order_type fields
        let (order_type_variant, order_type_price) = match &order.order_type {
            OrderType::Market => ("Market", None),
            OrderType::Limit { price } => ("Limit", Some(*price)),
        };

        // Extract status fields
        let (status_variant, status_filled_quantity, status_date) = match &order.status {
            OrderStatus::Pending => ("Pending", None, None),
            OrderStatus::PartiallyFilled { filled_quantity } => {
                ("PartiallyFilled", Some(*filled_quantity as i64), None)
            }
            OrderStatus::Filled { date } => ("Filled", Some(order.quantity as i64), Some(*date)),
            OrderStatus::Rejected { date } => ("Rejected", None, Some(*date)),
            OrderStatus::Cancelled { date } => ("Cancelled", None, Some(*date)),
        };

        let order_side_str = match order.order_side {
            OrderSide::Buy => "Buy",
            OrderSide::Sell => "Sell",
        };

        let result = sqlx::query(
            r#"
            UPDATE orders
            SET 
                client_id = $2,
                date = $3,
                symbol = $4,
                quantity = $5,
                order_side = $6,
                order_type_variant = $7,
                order_type_price = $8,
                status_variant = $9,
                status_filled_quantity = $10,
                status_date = $11,
                filled_price = $12
            WHERE id = $1
            "#,
        )
        .bind(order_id)
        .bind(order.client_id)
        .bind(order.date)
        .bind(&order.symbol)
        .bind(order.quantity as i64)
        .bind(order_side_str)
        .bind(order_type_variant)
        .bind(order_type_price)
        .bind(status_variant)
        .bind(status_filled_quantity)
        .bind(status_date)
        .bind(order.filled_price)
        .execute(&self.pool)
        .await
        .map_err(|e| OrderError::Repository(DbError::SqlxError(e)))?;

        if result.rows_affected() == 0 {
            return Err(OrderError::OrderNotFound);
        }

        Ok(())
    }

    pub async fn update_order_status(
        &self,
        order_id: &OrderId,
        status: OrderStatus,
    ) -> Result<(), OrderError> {
        debug!("Updating order {} status to {:?}", order_id, status);

        // Extract status fields
        let (status_variant, status_filled_quantity, status_date) = match &status {
            OrderStatus::Pending => ("Pending", None, None),
            OrderStatus::PartiallyFilled { filled_quantity } => {
                ("PartiallyFilled", Some(*filled_quantity as i64), None)
            }
            OrderStatus::Filled { date } => {
                // For Filled status, we need to get the order quantity
                // This is a simplified approach - in production you might handle this differently
                let order = self
                    .get_order_by_id(order_id)
                    .await?
                    .ok_or(OrderError::OrderNotFound)?;
                ("Filled", Some(order.quantity as i64), Some(*date))
            }
            OrderStatus::Rejected { date } => ("Rejected", None, Some(*date)),
            OrderStatus::Cancelled { date } => ("Cancelled", None, Some(*date)),
        };

        let result = sqlx::query(
            r#"
            UPDATE orders
            SET 
                status_variant = $2,
                status_filled_quantity = $3,
                status_date = $4
            WHERE id = $1
            "#,
        )
        .bind(order_id)
        .bind(status_variant)
        .bind(status_filled_quantity)
        .bind(status_date)
        .execute(&self.pool)
        .await
        .map_err(|e| OrderError::Repository(DbError::SqlxError(e)))?;

        if result.rows_affected() == 0 {
            return Err(OrderError::OrderNotFound);
        }

        Ok(())
    }
}
