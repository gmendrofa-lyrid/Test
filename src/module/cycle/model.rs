use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// One S&OP planning cycle. `status` governs whether the demand-side write tables accept edits.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Cycle {
        pub cycle_id: String,
        pub label: String,
        /// DV_TARGET window as a JSON array string (opaque to this service).
        pub target_months: String,
        pub status: String,
        pub opened_at: NaiveDateTime,
        pub frozen_at: Option<NaiveDateTime>,
        pub closed_at: Option<NaiveDateTime>,
}

/// POST body to ensure a cycle exists (idempotent). `target_months` is a JSON array string.
#[derive(Debug, Serialize, Deserialize)]
pub struct EnsureCycle {
        pub cycle_id: String,
        pub label: Option<String>,
        /// JSON array of YYYY-MM, e.g. `["2026-08","2026-09",...]`. Defaults to `[]`.
        pub target_months: Option<String>,
}

/// POST body to freeze a cycle (lock every consensus row and flip status to `frozen`).
#[derive(Debug, Serialize, Deserialize)]
pub struct FreezeCycle {
        pub cycle_id: String,
        pub frozen_by: String,
}
