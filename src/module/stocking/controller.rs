use std::collections::HashSet;
use std::ops::DerefMut;

use super::model::{CellUpsert, ChangeLogEntry, StockingCell, ValidatedOverrideValue};
use crate::utils::db;
use sqlx::Connection;

fn fmt_num(v: Option<f64>) -> Option<String> {
        v.map(|n| n.to_string())
}

const CELL_SELECT: &str = r#"SELECT item_code, branch, item_name, item_group, primary_item_group, uom,
          stocked, cls, uc, oh_qty, oh_value, lead_time, dd, md, aq, sd, obs, refreshed_at,
          sl_ovr, ot_ovr, moq_ovr, inc_ovr, sigma_lt_ovr, st_ovr, lt_ovr,
          override_updated_by, override_updated_at
   FROM stocking_policy WHERE item_code = ? AND branch = ? FOR UPDATE"#;

#[derive(Debug, serde::Serialize)]
pub struct BulkRefreshResult {
        pub upserted: u64,
        pub pruned: u64,
        pub retained_overrides: u64,
}

/// All cells, optionally only the stocked ones (the feed's default scope).
pub async fn get_cells(
        pool: &db::Pool,
        stocked_only: bool,
) -> Result<Vec<StockingCell>, sqlx::Error> {
        let only = i32::from(stocked_only);
        let rows = sqlx::query_as!(
                StockingCell,
                r#"SELECT item_code, branch, item_name, item_group, primary_item_group, uom,
                          stocked AS "stocked: bool", cls, uc, oh_qty, oh_value, lead_time,
                          dd, md, aq, sd, obs, refreshed_at,
                          sl_ovr, ot_ovr, moq_ovr, inc_ovr, sigma_lt_ovr,
                          st_ovr AS "st_ovr: bool", lt_ovr,
                          override_updated_by, override_updated_at
                   FROM stocking_policy
                   WHERE (? = 0 OR COALESCE(st_ovr, stocked) = 1)
                   ORDER BY item_code, branch"#,
                only
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}

pub async fn get_cell(
        pool: &db::Pool,
        item: &str,
        branch: &str,
) -> Result<Option<StockingCell>, sqlx::Error> {
        let row = sqlx::query_as!(
                StockingCell,
                r#"SELECT item_code, branch, item_name, item_group, primary_item_group, uom,
                          stocked AS "stocked: bool", cls, uc, oh_qty, oh_value, lead_time,
                          dd, md, aq, sd, obs, refreshed_at,
                          sl_ovr, ot_ovr, moq_ovr, inc_ovr, sigma_lt_ovr,
                          st_ovr AS "st_ovr: bool", lt_ovr,
                          override_updated_by, override_updated_at
                   FROM stocking_policy WHERE item_code = ? AND branch = ?"#,
                item,
                branch
        )
        .fetch_optional(pool)
        .await?;
        Ok(row)
}

/// Refresh write: atomically upsert ERP-refreshed columns, preserving every `*_ovr` column, then
/// reconcile rows no longer present in the full refresh payload. Stale rows without planner input
/// are pruned; rows with any override are retained but no longer reported as ERP-stocked.
pub async fn bulk_upsert_cells(
        pool: &db::Pool,
        cells: &[CellUpsert],
) -> Result<BulkRefreshResult, sqlx::Error> {
        // An empty payload is much more likely to mean an upstream outage than a legitimate full
        // refresh. Treat it as a safe no-op so it cannot wipe the materialized policy.
        if cells.is_empty() {
                return Ok(BulkRefreshResult {
                        upserted: 0,
                        pruned: 0,
                        retained_overrides: 0,
                });
        }

        let mut conn = pool.get().await.map_err(|_| sqlx::Error::PoolTimedOut)?;
        let mut tx = conn.deref_mut().begin().await?;
        let mut n = 0u64;
        let incoming: HashSet<(&str, &str)> = cells
                .iter()
                .map(|c| (c.item_code.as_str(), c.branch.as_str()))
                .collect();

        for c in cells {
                sqlx::query!(
                        r#"INSERT INTO stocking_policy
                               (item_code, branch, item_name, item_group, primary_item_group, uom,
                                stocked, cls, uc, oh_qty, oh_value, lead_time, dd, md, aq, sd, obs,
                                refreshed_at)
                           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?, NOW())
                           ON DUPLICATE KEY UPDATE
                               item_name=VALUES(item_name), item_group=VALUES(item_group),
                               primary_item_group=VALUES(primary_item_group), uom=VALUES(uom),
                               stocked=VALUES(stocked), cls=VALUES(cls), uc=VALUES(uc),
                               oh_qty=VALUES(oh_qty), oh_value=VALUES(oh_value),
                               lead_time=VALUES(lead_time), dd=VALUES(dd), md=VALUES(md),
                               aq=VALUES(aq), sd=VALUES(sd), obs=VALUES(obs), refreshed_at=NOW()"#,
                        c.item_code,
                        c.branch,
                        c.item_name,
                        c.item_group,
                        c.primary_item_group,
                        c.uom,
                        c.stocked,
                        c.cls,
                        c.uc,
                        c.oh_qty,
                        c.oh_value,
                        c.lead_time,
                        c.dd,
                        c.md,
                        c.aq,
                        c.sd,
                        c.obs
                )
                .execute(&mut *tx)
                .await?;
                n += 1;
        }

        let stored = sqlx::query_as::<_, (String, String, i64)>(
                r#"SELECT item_code, branch,
                          (sl_ovr IS NOT NULL OR ot_ovr IS NOT NULL OR moq_ovr IS NOT NULL
                           OR inc_ovr IS NOT NULL OR sigma_lt_ovr IS NOT NULL
                           OR st_ovr IS NOT NULL OR lt_ovr IS NOT NULL) AS has_override
                   FROM stocking_policy FOR UPDATE"#,
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut pruned = 0u64;
        let mut retained_overrides = 0u64;
        for (item, branch, has_override) in stored {
                if incoming.contains(&(item.as_str(), branch.as_str())) {
                        continue;
                }
                if has_override != 0 {
                        let result = sqlx::query(
                                "UPDATE stocking_policy SET stocked = FALSE WHERE item_code = ? AND branch = ?",
                        )
                        .bind(&item)
                        .bind(&branch)
                        .execute(&mut *tx)
                        .await?;
                        retained_overrides += result.rows_affected();
                } else {
                        let result = sqlx::query(
                                "DELETE FROM stocking_policy WHERE item_code = ? AND branch = ?",
                        )
                        .bind(&item)
                        .bind(&branch)
                        .execute(&mut *tx)
                        .await?;
                        pruned += result.rows_affected();
                }
        }

        tx.commit().await?;
        Ok(BulkRefreshResult {
                upserted: n,
                pruned,
                retained_overrides,
        })
}

/// Read the current stored override for one field, as a string (for the change-log `from`).
fn current_override(cell: &Option<StockingCell>, field: &str) -> Option<String> {
        let c = cell.as_ref()?;
        match field {
                "service_level" => fmt_num(c.sl_ovr),
                "order_type" => c.ot_ovr.clone(),
                "moq" => fmt_num(c.moq_ovr),
                "increment" => fmt_num(c.inc_ovr),
                "sigma_lt" => fmt_num(c.sigma_lt_ovr),
                "stocked" => c.st_ovr.map(|b| if b { "1" } else { "0" }.to_string()),
                "lead_time" => fmt_num(c.lt_ovr),
                _ => None,
        }
}

fn value_to_string(v: &ValidatedOverrideValue) -> Option<String> {
        match v {
                ValidatedOverrideValue::Text(s) => Some(s.clone()),
                ValidatedOverrideValue::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
                ValidatedOverrideValue::Number(n) => Some(n.to_string()),
                ValidatedOverrideValue::Clear => None,
        }
}

/// Set ONE field's override on a cell (creates the row if the refresh hasn't populated it), and log
/// the change. Only the given column is touched — other overrides + ERP columns are untouched.
pub async fn set_field_override(
        pool: &db::Pool,
        item: &str,
        branch: &str,
        field: &str,
        column: &str,
        value: &ValidatedOverrideValue,
        actor: &Option<String>,
) -> Result<StockingCell, sqlx::Error> {
        let mut conn = pool.get().await.map_err(|_| sqlx::Error::PoolTimedOut)?;
        let mut tx = conn.deref_mut().begin().await?;
        let existing = sqlx::query_as::<_, StockingCell>(CELL_SELECT)
                .bind(item)
                .bind(branch)
                .fetch_optional(&mut *tx)
                .await?;
        let from = current_override(&existing, field);
        let to = value_to_string(value);

        // `column` is whitelisted upstream (FieldOverride::column), so this interpolation is safe.
        let sql = format!(
                "INSERT INTO stocking_policy (item_code, branch, {column}, override_updated_by, \
                 override_updated_at) VALUES (?, ?, ?, ?, NOW()) \
                 ON DUPLICATE KEY UPDATE {column}=VALUES({column}), \
                 override_updated_by=VALUES(override_updated_by), override_updated_at=NOW()"
        );
        let q = sqlx::query(&sql).bind(item).bind(branch);
        let q = match value {
                ValidatedOverrideValue::Text(v) => q.bind(Some(v)),
                ValidatedOverrideValue::Bool(v) => q.bind(Some(v)),
                ValidatedOverrideValue::Number(v) => q.bind(Some(v)),
                ValidatedOverrideValue::Clear if field == "order_type" => q.bind(None::<String>),
                ValidatedOverrideValue::Clear if field == "stocked" => q.bind(None::<bool>),
                ValidatedOverrideValue::Clear => q.bind(None::<f64>),
        };
        q.bind(actor.clone()).execute(&mut *tx).await?;

        if from != to {
                sqlx::query!(
                        r#"INSERT INTO policy_change_log
                               (id, item_code, branch, field, from_value, to_value, actor)
                           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
                        uuid::Uuid::new_v4().to_string(),
                        item,
                        branch,
                        field,
                        from,
                        to,
                        actor
                )
                .execute(&mut *tx)
                .await?;
        }

        let result = sqlx::query_as::<_, StockingCell>(CELL_SELECT)
                .bind(item)
                .bind(branch)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?;
        tx.commit().await?;
        Ok(result)
}

/// Clear and audit every populated override on a cell as one atomic operation.
pub async fn clear_override(
        pool: &db::Pool,
        item: &str,
        branch: &str,
        actor: &Option<String>,
) -> Result<u64, sqlx::Error> {
        let mut conn = pool.get().await.map_err(|_| sqlx::Error::PoolTimedOut)?;
        let mut tx = conn.deref_mut().begin().await?;
        let existing = sqlx::query_as::<_, StockingCell>(CELL_SELECT)
                .bind(item)
                .bind(branch)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(cell) = existing else {
                tx.commit().await?;
                return Ok(0);
        };

        let fields = [
                ("service_level", fmt_num(cell.sl_ovr)),
                ("order_type", cell.ot_ovr),
                ("moq", fmt_num(cell.moq_ovr)),
                ("increment", fmt_num(cell.inc_ovr)),
                ("sigma_lt", fmt_num(cell.sigma_lt_ovr)),
                (
                        "stocked",
                        cell.st_ovr.map(|b| if b { "1" } else { "0" }.to_string()),
                ),
                ("lead_time", fmt_num(cell.lt_ovr)),
        ];

        let res = sqlx::query!(
                r#"UPDATE stocking_policy
                   SET sl_ovr=NULL, ot_ovr=NULL, moq_ovr=NULL, inc_ovr=NULL, sigma_lt_ovr=NULL,
                       st_ovr=NULL, lt_ovr=NULL, override_updated_by=?, override_updated_at=NOW()
                   WHERE item_code = ? AND branch = ?
                     AND (sl_ovr IS NOT NULL OR ot_ovr IS NOT NULL OR moq_ovr IS NOT NULL
                          OR inc_ovr IS NOT NULL OR sigma_lt_ovr IS NOT NULL
                          OR st_ovr IS NOT NULL OR lt_ovr IS NOT NULL)"#,
                actor,
                item,
                branch
        )
        .execute(&mut *tx)
        .await?;

        for (field, from) in fields.into_iter().filter(|(_, from)| from.is_some()) {
                sqlx::query!(
                        r#"INSERT INTO policy_change_log
                               (id, item_code, branch, field, from_value, to_value, actor)
                           VALUES (?, ?, ?, ?, ?, NULL, ?)"#,
                        uuid::Uuid::new_v4().to_string(),
                        item,
                        branch,
                        field,
                        from,
                        actor
                )
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(res.rows_affected())
}

pub async fn get_changelog(
        pool: &db::Pool,
        item: &str,
) -> Result<Vec<ChangeLogEntry>, sqlx::Error> {
        let rows = sqlx::query_as!(
                ChangeLogEntry,
                r#"SELECT id, item_code, branch, field, from_value, to_value, actor, created_at
                   FROM policy_change_log
                   WHERE (? = '' OR item_code = ?)
                   ORDER BY created_at DESC LIMIT 500"#,
                item,
                item
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}
