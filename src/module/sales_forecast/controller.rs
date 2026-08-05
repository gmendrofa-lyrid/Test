use super::model::{SalesForecastEntry, SalesForecastRow};
use crate::module::cycle;
use crate::utils::db;

/// Why a write was rejected before it touched the table.
#[derive(Debug)]
pub enum WriteError {
        /// The cycle does not exist.
        NoCycle,
        /// The cycle is frozen/closed — writes are not accepted.
        Locked(String),
        Db(sqlx::Error),
}

impl From<sqlx::Error> for WriteError {
        fn from(e: sqlx::Error) -> Self {
                WriteError::Db(e)
        }
}

/// Reject the write unless the cycle exists and is `open` (mirrors the frontend's assertOpen).
async fn require_open(pool: &db::Pool, cycle_id: &str) -> Result<(), WriteError> {
        match cycle::controller::status(pool, cycle_id).await? {
                None => Err(WriteError::NoCycle),
                Some(s) if s == "open" => Ok(()),
                Some(s) => Err(WriteError::Locked(s)),
        }
}

/// Upsert every entry for the cycle; returns the number of rows written.
pub async fn save(
        pool: &db::Pool,
        cycle_id: &str,
        updated_by: &str,
        entries: &[SalesForecastEntry],
) -> Result<u64, WriteError> {
        require_open(pool, cycle_id).await?;

        let mut n: u64 = 0;
        for e in entries {
                sqlx::query!(
                        r#"INSERT INTO sales_forecast_entry
                               (cycle_id, salesperson, customer, item_code, branch, ym,
                                sales_qty, sales_reason, selected, updated_by, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())
                           ON DUPLICATE KEY UPDATE
                               salesperson  = VALUES(salesperson),
                               sales_qty    = VALUES(sales_qty),
                               sales_reason = VALUES(sales_reason),
                               selected     = VALUES(selected),
                               updated_by   = VALUES(updated_by),
                               updated_at   = NOW()"#,
                        cycle_id,
                        e.salesperson,
                        e.customer,
                        e.item_code,
                        e.branch,
                        e.ym,
                        e.sales_qty,
                        e.sales_reason,
                        e.selected,
                        updated_by
                )
                .execute(pool)
                .await?;
                n += 1;
        }
        Ok(n)
}

/// All rep Sales-Forecast entries for a cycle (optionally filtered to one salesperson).
pub async fn load(
        pool: &db::Pool,
        cycle_id: &str,
        salesperson: &str,
) -> Result<Vec<SalesForecastRow>, sqlx::Error> {
        let rows = sqlx::query_as!(
                SalesForecastRow,
                r#"SELECT salesperson, customer, item_code, branch, ym,
                          sales_qty, sales_reason, selected, updated_by, updated_at
                   FROM sales_forecast_entry
                   WHERE cycle_id = ? AND (? = '' OR salesperson = ?)
                   ORDER BY salesperson, customer, item_code, branch, ym"#,
                cycle_id,
                salesperson,
                salesperson
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}
