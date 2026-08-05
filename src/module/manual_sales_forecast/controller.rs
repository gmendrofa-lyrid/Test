use super::model::ManualSalesForecastRow;
use crate::utils::db;

/// All imported manual Sales-Forecast rows for a cycle (optionally filtered to one salesperson).
pub async fn load(
        pool: &db::Pool,
        cycle_id: &str,
        salesperson: &str,
) -> Result<Vec<ManualSalesForecastRow>, sqlx::Error> {
        let rows = sqlx::query_as!(
                ManualSalesForecastRow,
                r#"SELECT salesperson, customer, item_code, branch, ym,
                          sales_qty, sales_reason, selected, updated_by, updated_at
                   FROM manual_sales_forecast_entry
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
