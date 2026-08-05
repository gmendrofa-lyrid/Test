use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// One persisted rep Sales-Forecast cell (customer × item × branch × month).
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SalesForecastRow {
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

/// One entry inside a save request.
#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastEntry {
        pub salesperson: String,
        pub customer: String,
        pub item_code: String,
        pub branch: String,
        pub ym: String,
        pub sales_qty: Option<f64>,
        pub sales_reason: Option<String>,
        pub selected: Option<String>,
}

/// POST body: save a batch of rep Sales-Forecast entries for a cycle.
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveSalesForecast {
        pub cycle_id: String,
        pub updated_by: String,
        pub entries: Vec<SalesForecastEntry>,
}
