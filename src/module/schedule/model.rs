use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JobSchedule {
        pub job_key: String,
        pub enabled: bool,
        pub interval_months: u16,
        pub interval_hours: Option<u16>,
        pub next_run_at: DateTime<Utc>,
        pub retry_after_at: Option<DateTime<Utc>>,
        pub status: String,
        pub last_run_started_at: Option<DateTime<Utc>>,
        pub last_run_finished_at: Option<DateTime<Utc>>,
        pub last_success_at: Option<DateTime<Utc>>,
        pub lease_owner: Option<String>,
        pub lease_expires_at: Option<DateTime<Utc>>,
        pub last_error: Option<String>,
        pub row_version: u64,
        pub updated_by: Option<String>,
        pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ClaimResult {
        pub claimed: bool,
        pub schedule: Option<JobSchedule>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimJob {
        pub owner: String,
}

#[derive(Debug, Deserialize)]
pub struct FinishJob {
        pub owner: String,
        pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSchedule {
        pub enabled: Option<bool>,
        pub interval_months: Option<u16>,
        pub next_run_at: Option<DateTime<Utc>>,
        pub actor: Option<String>,
}
