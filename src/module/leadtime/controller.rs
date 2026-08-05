use super::model::{PmLeadTimeDefault, UpsertPmLeadTime};
use crate::utils::db;

pub async fn get_all(pool: &db::Pool) -> Result<Vec<PmLeadTimeDefault>, sqlx::Error> {
        let rows = sqlx::query_as!(
                PmLeadTimeDefault,
                r#"SELECT id, principal, pig, item_group, leg, lead_days, sigma_days, note,
                          active AS "active: bool", updated_by, updated_at
                   FROM pm_lead_time_default
                   ORDER BY principal, pig, item_group, leg"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}

/// Upsert one PM lead-time default row, keyed by (principal, pig, item_group, leg). Returns it.
pub async fn upsert(
        pool: &db::Pool,
        body: &UpsertPmLeadTime,
) -> Result<PmLeadTimeDefault, sqlx::Error> {
        let principal = body.principal.clone().unwrap_or_default();
        let pig = body.pig.clone().unwrap_or_default();
        let item_group = body.item_group.clone().unwrap_or_default();
        sqlx::query!(
                r#"INSERT INTO pm_lead_time_default
                       (id, principal, pig, item_group, leg, lead_days, sigma_days, note, active, updated_by)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON DUPLICATE KEY UPDATE
                       lead_days  = VALUES(lead_days),
                       sigma_days = VALUES(sigma_days),
                       note       = VALUES(note),
                       active     = VALUES(active),
                       updated_by = VALUES(updated_by)"#,
                uuid::Uuid::new_v4().to_string(),
                principal,
                pig,
                item_group,
                body.leg,
                body.lead_days.unwrap_or(0.0),
                body.sigma_days.unwrap_or(0.0),
                body.note,
                body.active.unwrap_or(true),
                body.actor
        )
        .execute(pool)
        .await?;

        let row = sqlx::query_as!(
                PmLeadTimeDefault,
                r#"SELECT id, principal, pig, item_group, leg, lead_days, sigma_days, note,
                          active AS "active: bool", updated_by, updated_at
                   FROM pm_lead_time_default
                   WHERE principal = ? AND pig = ? AND item_group = ? AND leg = ?"#,
                principal,
                pig,
                item_group,
                body.leg
        )
        .fetch_optional(pool)
        .await?;
        row.ok_or(sqlx::Error::RowNotFound)
}

/// Delete one PM lead-time default by its id. Returns the number of rows removed.
pub async fn delete(pool: &db::Pool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM pm_lead_time_default WHERE id = ?", id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected())
}
