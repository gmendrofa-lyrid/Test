use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// One persisted DSP consensus pick (customer × item × branch × month).
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ConsensusRow {
        pub customer: String,
        pub item_code: String,
        pub branch: String,
        pub ym: String,
        pub selected_source: String,
        pub selected_qty: f64,
        pub reason: Option<String>,
        pub auto_accepted: bool,
        pub is_frozen: bool,
}

/// One pick inside a save request.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusPick {
        pub customer: String,
        pub item_code: String,
        pub branch: String,
        pub ym: String,
        pub selected_source: String,
        pub selected_qty: f64,
        pub reason: Option<String>,
        #[serde(default)]
        pub auto_accepted: bool,
}

/// POST body: save a batch of DSP consensus picks for a cycle.
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveConsensus {
        pub cycle_id: String,
        pub resolved_by: String,
        pub picks: Vec<ConsensusPick>,
}
