use super::model::{FeedSnapshot, Snapshot};
use crate::utils::db;

pub async fn get(pool: &db::Pool, as_of: &str) -> Result<Option<Snapshot>, sqlx::Error> {
        let row = sqlx::query_as!(
                Snapshot,
                r#"SELECT as_of, payload, computed_at FROM demand_snapshot WHERE as_of = ?"#,
                as_of
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
}

/// Replace the snapshot for a date (write-through cache from the cockpit's cold start).
pub async fn upsert(pool: &db::Pool, as_of: &str, payload: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
                r#"INSERT INTO demand_snapshot (as_of, payload)
                   VALUES (?, ?)
                   ON DUPLICATE KEY UPDATE payload = VALUES(payload)"#,
                as_of,
                payload
        )
        .execute(pool)
        .await?;
        Ok(())
}

/// Read a generic materialized-feed snapshot by key (e.g. "principal").
pub async fn get_feed(pool: &db::Pool, key: &str) -> Result<Option<FeedSnapshot>, sqlx::Error> {
        let row = sqlx::query_as!(
                FeedSnapshot,
                r#"SELECT snapshot_key, payload, computed_at FROM feed_snapshot WHERE snapshot_key = ?"#,
                key
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
}

/// Write-through a generic materialized-feed snapshot (computed by the scheduled refresh).
pub async fn upsert_feed(pool: &db::Pool, key: &str, payload: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
                r#"INSERT INTO feed_snapshot (snapshot_key, payload)
                   VALUES (?, ?)
                   ON DUPLICATE KEY UPDATE payload = VALUES(payload)"#,
                key,
                payload
        )
        .execute(pool)
        .await?;
        Ok(())
}
