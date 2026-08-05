use super::model::{ClaimResult, JobSchedule, UpdateSchedule};
use crate::utils::db;

const SELECT_SCHEDULE: &str = r#"
        SELECT job_key, enabled, interval_months, interval_hours, next_run_at, retry_after_at, status,
               last_run_started_at, last_run_finished_at, last_success_at,
               lease_owner, lease_expires_at, last_error, row_version, updated_by, updated_at
        FROM job_schedule
        WHERE job_key = ?
"#;

pub async fn get(pool: &db::Pool, key: &str) -> Result<Option<JobSchedule>, sqlx::Error> {
        sqlx::query_as::<_, JobSchedule>(SELECT_SCHEDULE)
                .bind(key)
                .fetch_optional(pool)
                .await
}

pub async fn update(
        pool: &db::Pool,
        key: &str,
        body: &UpdateSchedule,
) -> Result<Option<JobSchedule>, sqlx::Error> {
        sqlx::query(
                r#"UPDATE job_schedule
                       SET enabled = COALESCE(?, enabled),
                           interval_months = COALESCE(?, interval_months),
                           next_run_at = COALESCE(?, next_run_at),
                       status = CASE
                           WHEN COALESCE(?, enabled) = FALSE THEN 'disabled'
                           WHEN status = 'disabled' THEN 'idle'
                           ELSE status
                       END,
                       updated_by = COALESCE(?, updated_by),
                       row_version = row_version + 1
                   WHERE job_key = ?"#,
        )
        .bind(body.enabled)
        .bind(body.interval_months)
        .bind(body.next_run_at)
        .bind(body.enabled)
        .bind(&body.actor)
        .bind(key)
        .execute(pool)
        .await?;

        get(pool, key).await
}

/// Atomically acquire a one-hour lease when this job is due. Only one caller can change the row,
/// so multiple Next.js workers can poll safely without launching duplicate ERP refreshes.
pub async fn claim(pool: &db::Pool, key: &str, owner: &str) -> Result<ClaimResult, sqlx::Error> {
        let result = sqlx::query(
                r#"UPDATE job_schedule
                   SET status = 'running',
                       last_run_started_at = UTC_TIMESTAMP(6),
                       last_run_finished_at = NULL,
                       lease_owner = ?,
                       lease_expires_at = DATE_ADD(UTC_TIMESTAMP(6), INTERVAL 60 MINUTE),
                       last_error = NULL,
                       updated_by = ?,
                       row_version = row_version + 1
                   WHERE job_key = ?
                     AND enabled = TRUE
                     AND next_run_at <= UTC_TIMESTAMP(6)
                     AND (retry_after_at IS NULL OR retry_after_at <= UTC_TIMESTAMP(6))
                     AND (
                         status <> 'running'
                         OR lease_expires_at IS NULL
                         OR lease_expires_at <= UTC_TIMESTAMP(6)
                     )"#,
        )
        .bind(owner)
        .bind(owner)
        .bind(key)
        .execute(pool)
        .await?;

        Ok(ClaimResult {
                claimed: result.rows_affected() == 1,
                schedule: get(pool, key).await?,
        })
}

/// Mark a successful run and schedule the next one. Jobs with `interval_hours` use a rolling
/// operational cadence; older jobs retain their calendar-month cadence at midnight Jakarta.
pub async fn complete(pool: &db::Pool, key: &str, owner: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
                r#"UPDATE job_schedule
                   SET status = 'idle',
                       last_run_finished_at = UTC_TIMESTAMP(6),
                       last_success_at = UTC_TIMESTAMP(6),
                       next_run_at = CASE
                           WHEN interval_hours IS NOT NULL
                               THEN DATE_ADD(
                                   next_run_at,
                                   INTERVAL (
                                       GREATEST(
                                           1,
                                           FLOOR(
                                               TIMESTAMPDIFF(
                                                   HOUR,
                                                   next_run_at,
                                                   UTC_TIMESTAMP(6)
                                               ) / interval_hours
                                           ) + 1
                                       ) * interval_hours
                                   ) HOUR
                               )
                           ELSE CONVERT_TZ(
                               DATE_ADD(
                                   DATE(CONVERT_TZ(
                                       UTC_TIMESTAMP(6),
                                       '+00:00',
                                       '+07:00'
                                   )),
                                   INTERVAL interval_months MONTH
                               ),
                               '+07:00',
                               @@session.time_zone
                           )
                       END,
                       retry_after_at = NULL,
                       lease_owner = NULL,
                       lease_expires_at = NULL,
                       last_error = NULL,
                       updated_by = ?,
                       row_version = row_version + 1
                   WHERE job_key = ?
                     AND status = 'running'
                     AND lease_owner = ?"#,
        )
        .bind(owner)
        .bind(key)
        .bind(owner)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
}

/// Release the lease but keep `next_run_at` due. The hourly poll can retry rather than waiting six
/// months after a failed ERP connection.
pub async fn fail(
        pool: &db::Pool,
        key: &str,
        owner: &str,
        error: &str,
) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
                r#"UPDATE job_schedule
                   SET status = 'failed',
                       last_run_finished_at = UTC_TIMESTAMP(6),
                       retry_after_at = DATE_ADD(UTC_TIMESTAMP(6), INTERVAL 1 HOUR),
                       lease_owner = NULL,
                       lease_expires_at = NULL,
                       last_error = ?,
                       updated_by = ?,
                       row_version = row_version + 1
                   WHERE job_key = ?
                     AND status = 'running'
                     AND lease_owner = ?"#,
        )
        .bind(error)
        .bind(owner)
        .bind(key)
        .bind(owner)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
}
