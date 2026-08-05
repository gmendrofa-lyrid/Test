use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// One row of the imported manual Sales-Forecast (master_fc_sales.xlsx FORECAST sheet).
/// Same shape as `sales_forecast_entry`.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ManualSalesForecastRow {
        pub salesperson: String,
        pub customer: String,
        pub item_code: String,
        pub branch: String,
        pub ym: String,
        pub sales_qty: Option<f64>,
        pub sales_reason: Option<String>,
        pub selected: Option<String>,
        pub updated_by: String,
        pub updated_at: NaiveDateTime,
}
