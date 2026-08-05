use super::model::Cycle;
use crate::utils::db;
use sqlx::Connection;
use std::collections::HashSet;

/// Fetch one cycle by id.
pub async fn get(pool: &db::Pool, cycle_id: &str) -> Result<Option<Cycle>, sqlx::Error> {
        let row = sqlx::query_as!(
                Cycle,
                r#"SELECT cycle_id, label, CAST(target_months AS CHAR) AS `target_months!`,
                          status, opened_at, frozen_at, closed_at
                   FROM sop_cycle WHERE cycle_id = ?"#,
                cycle_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
}

/// The status string of a cycle, or None if it does not exist.
pub async fn status(pool: &db::Pool, cycle_id: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query!(
                r#"SELECT status FROM sop_cycle WHERE cycle_id = ?"#,
                cycle_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|r| r.status))
}

/// Ensure a cycle row exists (open). Idempotent — an existing cycle is left untouched.
pub async fn ensure(
        pool: &db::Pool,
        cycle_id: &str,
        label: Option<&str>,
        target_months: Option<&str>,
) -> Result<Cycle, sqlx::Error> {
        let label = label.unwrap_or(cycle_id);
        let target_months = target_months.unwrap_or("[]");
        sqlx::query!(
                r#"INSERT INTO sop_cycle (cycle_id, label, target_months)
                   VALUES (?, ?, ?)
                   ON DUPLICATE KEY UPDATE cycle_id = cycle_id"#,
                cycle_id,
                label,
                target_months
        )
        .execute(pool)
        .await?;
        let saved = get(pool, cycle_id).await?;
        saved.ok_or(sqlx::Error::RowNotFound)
}

/// Freeze a cycle atomically.
///
/// The cycle row is locked first so concurrent consensus saves serialize with this operation.
/// Every configured target month must have at least one consensus row before the rows and cycle
/// are frozen in the same transaction. Repeating a successful freeze is a no-op.
pub async fn freeze(pool: &db::Pool, cycle_id: &str, frozen_by: &str) -> Result<u64, FreezeError> {
        let mut connection = pool
                .get()
                .await
                .map_err(|_| FreezeError::Db(sqlx::Error::PoolTimedOut))?;
        let mut transaction = connection.begin().await?;

        let cycle = sqlx::query!(
                r#"SELECT status, CAST(target_months AS CHAR) AS `target_months!`
                   FROM sop_cycle
                   WHERE cycle_id = ?
                   FOR UPDATE"#,
                cycle_id
        )
        .fetch_optional(&mut *transaction)
        .await?;

        let cycle = match cycle {
                None => return Err(FreezeError::NotFound),
                Some(cycle) if cycle.status == "closed" => return Err(FreezeError::Closed),
                Some(cycle) if cycle.status == "frozen" => {
                        transaction.commit().await?;
                        return Ok(0);
                }
                Some(cycle) => cycle,
        };

        let target_months: Vec<String> = serde_json::from_str(&cycle.target_months)
                .map_err(|_| FreezeError::InvalidTargetMonths)?;
        let target_months: HashSet<String> = target_months.into_iter().collect();
        if target_months.is_empty() {
                return Err(FreezeError::InvalidTargetMonths);
        }

        let covered_months = sqlx::query!(
                r#"SELECT ym
                   FROM demand_consensus
                   WHERE cycle_id = ?
                   GROUP BY ym"#,
                cycle_id
        )
        .fetch_all(&mut *transaction)
        .await?;
        if covered_months.is_empty() {
                return Err(FreezeError::Empty);
        }

        let covered_months: HashSet<String> =
                covered_months.into_iter().map(|row| row.ym).collect();
        let mut missing_months: Vec<String> =
                target_months.difference(&covered_months).cloned().collect();
        missing_months.sort();
        if !missing_months.is_empty() {
                return Err(FreezeError::Incomplete { missing_months });
        }

        let locked = sqlx::query!(
                r#"UPDATE demand_consensus
                      SET is_frozen = 1, frozen_by = ?, frozen_at = NOW()
                    WHERE cycle_id = ? AND is_frozen = 0"#,
                frozen_by,
                cycle_id
        )
        .execute(&mut *transaction)
        .await?
        .rows_affected();

        sqlx::query!(
                r#"UPDATE sop_cycle
                      SET status = 'frozen', frozen_at = NOW()
                    WHERE cycle_id = ? AND status = 'open'"#,
                cycle_id
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(locked)
}

/// Outcome of a rejected freeze — mapped to HTTP by the API layer.
#[derive(Debug)]
pub enum FreezeError {
        NotFound,
        Closed,
        Empty,
        Incomplete { missing_months: Vec<String> },
        InvalidTargetMonths,
        Db(sqlx::Error),
}

impl From<sqlx::Error> for FreezeError {
        fn from(e: sqlx::Error) -> Self {
                FreezeError::Db(e)
        }
}
