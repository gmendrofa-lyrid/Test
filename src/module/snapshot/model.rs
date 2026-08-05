use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Snapshot {
        pub as_of: NaiveDate,
        /// Opaque JSON owned by the cockpit frontend ({ stocked, series }).
        pub payload: String,
        pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutSnapshot {
        pub as_of: String,
        pub payload: String,
}

/// Generic materialized-feed cache row (keyed by an arbitrary snapshot key, not a date).
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FeedSnapshot {
        pub snapshot_key: String,
        pub payload: String,
        pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutFeedSnapshot {
        pub payload: String,
}
