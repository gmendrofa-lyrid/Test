use super::model::{ConsensusPick, ConsensusRow};
use crate::module::sales_forecast::controller::WriteError;
use crate::utils::db;
use sqlx::Connection;

/// Save DSP picks. A pick whose row is already frozen is skipped — a frozen consensus is
/// plan-of-record and must not be overwritten. The cycle row lock serializes this batch with
/// freezing, preventing a save that observed `open` from committing behind a completed freeze.
/// Returns the number of rows written.
pub async fn save(
        pool: &db::Pool,
        cycle_id: &str,
        resolved_by: &str,
        picks: &[ConsensusPick],
) -> Result<u64, WriteError> {
        let mut connection = pool
                .get()
                .await
                .map_err(|_| WriteError::Db(sqlx::Error::PoolTimedOut))?;
        let mut transaction = connection.begin().await?;
        let cycle = sqlx::query!(
                r#"SELECT status
                   FROM sop_cycle
                   WHERE cycle_id = ?
                   FOR UPDATE"#,
                cycle_id
        )
        .fetch_optional(&mut *transaction)
        .await?;
        match cycle {
                None => return Err(WriteError::NoCycle),
                Some(cycle) if cycle.status == "open" => {}
                Some(cycle) => return Err(WriteError::Locked(cycle.status)),
        }

        let mut n: u64 = 0;
        for p in picks {
                let frozen = sqlx::query!(
                        r#"SELECT is_frozen AS `is_frozen: bool` FROM demand_consensus
                           WHERE cycle_id = ? AND customer = ? AND item_code = ? AND branch = ? AND ym = ?"#,
                        cycle_id,
                        p.customer,
                        p.item_code,
                        p.branch,
                        p.ym
                )
                .fetch_optional(&mut *transaction)
                .await?
                .map(|r| r.is_frozen)
                .unwrap_or(false);
                if frozen {
                        continue;
                }

                sqlx::query!(
                        r#"INSERT INTO demand_consensus
                               (cycle_id, customer, item_code, branch, ym,
                                selected_source, selected_qty, reason, auto_accepted, resolved_by, resolved_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())
                           ON DUPLICATE KEY UPDATE
                               selected_source = VALUES(selected_source),
                               selected_qty    = VALUES(selected_qty),
                               reason          = VALUES(reason),
                               auto_accepted   = VALUES(auto_accepted),
                               resolved_by     = VALUES(resolved_by),
                               resolved_at     = NOW()"#,
                        cycle_id,
                        p.customer,
                        p.item_code,
                        p.branch,
                        p.ym,
                        p.selected_source,
                        p.selected_qty,
                        p.reason,
                        p.auto_accepted,
                        resolved_by
                )
                .execute(&mut *transaction)
                .await?;
                n += 1;
        }
        transaction.commit().await?;
        Ok(n)
}

/// All consensus picks for a cycle.
pub async fn load(pool: &db::Pool, cycle_id: &str) -> Result<Vec<ConsensusRow>, sqlx::Error> {
        let rows = sqlx::query_as!(
                ConsensusRow,
                r#"SELECT customer, item_code, branch, ym, selected_source, selected_qty,
                          reason, auto_accepted AS `auto_accepted: bool`, is_frozen AS `is_frozen: bool`
                   FROM demand_consensus
                   WHERE cycle_id = ?
                   ORDER BY customer, item_code, branch, ym"#,
                cycle_id
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}
