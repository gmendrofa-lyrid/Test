use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct EoqParam {
        pub id: String,
        /// '' = company-wide default; otherwise an item-group override.
        pub item_group: String,
        pub ordering_cost: Option<f64>,
        pub holding_pct: Option<f64>,
        pub active: bool,
        pub updated_by: Option<String>,
        pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertEoq {
        pub item_group: Option<String>,
        pub ordering_cost: Option<f64>,
        pub holding_pct: Option<f64>,
        pub active: Option<bool>,
        pub actor: Option<String>,
}
