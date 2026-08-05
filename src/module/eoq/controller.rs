use super::model::{EoqParam, UpsertEoq};
use crate::utils::db;

pub async fn get_all(pool: &db::Pool) -> Result<Vec<EoqParam>, sqlx::Error> {
        let rows = sqlx::query_as!(
                EoqParam,
                r#"SELECT id, item_group, ordering_cost, holding_pct,
                          active AS "active: bool", updated_by, updated_at
                   FROM eoq_param ORDER BY item_group"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}

/// Upsert the EOQ params for an item group ('' = company-wide default). Returns the row.
pub async fn upsert(pool: &db::Pool, body: &UpsertEoq) -> Result<EoqParam, sqlx::Error> {
        let group = body.item_group.clone().unwrap_or_default();
        sqlx::query!(
                r#"INSERT INTO eoq_param
                       (id, item_group, ordering_cost, holding_pct, active, updated_by)
                   VALUES (?, ?, ?, ?, ?, ?)
                   ON DUPLICATE KEY UPDATE
                       ordering_cost = VALUES(ordering_cost),
                       holding_pct   = VALUES(holding_pct),
                       active        = VALUES(active),
                       updated_by    = VALUES(updated_by)"#,
                uuid::Uuid::new_v4().to_string(),
                group,
                body.ordering_cost,
                body.holding_pct,
                body.active.unwrap_or(false),
                body.actor
        )
        .execute(pool)
        .await?;

        let row = sqlx::query_as!(
                EoqParam,
                r#"SELECT id, item_group, ordering_cost, holding_pct,
                          active AS "active: bool", updated_by, updated_at
                   FROM eoq_param WHERE item_group = ?"#,
                group
        )
        .fetch_optional(pool)
        .await?;
        row.ok_or(sqlx::Error::RowNotFound)
}
