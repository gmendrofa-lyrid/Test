use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ConfigRow {
        pub config_key: String,
        pub config_value: String,
        pub updated_by: Option<String>,
        pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetConfig {
        pub value: String,
        pub actor: Option<String>,
}
