use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// The five lead-time legs a PM default may be set against (config.py:140).
pub const LEGS: [&str; 5] = [
        "supplier_dispatch",
        "ocean_air_transit",
        "customs",
        "bpom_permit",
        "qc_release",
];

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PmLeadTimeDefault {
        pub id: String,
        pub principal: String,
        pub pig: String,
        pub item_group: String,
        pub leg: String,
        pub lead_days: f64,
        pub sigma_days: f64,
        pub note: Option<String>,
        pub active: bool,
        pub updated_by: Option<String>,
        pub updated_at: DateTime<Utc>,
}

/// Upsert body — the four grain keys identify the row; '' means "any" at that axis.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertPmLeadTime {
        pub principal: Option<String>,
        pub pig: Option<String>,
        pub item_group: Option<String>,
        pub leg: String,
        pub lead_days: Option<f64>,
        pub sigma_days: Option<f64>,
        pub note: Option<String>,
        pub active: Option<bool>,
        pub actor: Option<String>,
}

impl UpsertPmLeadTime {
        /// Reject a leg that is not one of the five known legs; keep the store clean.
        pub fn valid_leg(&self) -> bool {
                LEGS.contains(&self.leg.as_str())
        }
}
