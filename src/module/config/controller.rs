use super::model::{ConfigRow, SetConfig};
use crate::utils::db;

pub async fn get_all(pool: &db::Pool) -> Result<Vec<ConfigRow>, sqlx::Error> {
        let rows = sqlx::query_as!(
                ConfigRow,
                r#"SELECT config_key, config_value, updated_by, updated_at
                   FROM config ORDER BY config_key"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
}

/// Upsert one config key. Returns the persisted row.
pub async fn set(pool: &db::Pool, key: &str, body: &SetConfig) -> Result<ConfigRow, sqlx::Error> {
        sqlx::query!(
                r#"INSERT INTO config (config_key, config_value, updated_by)
                   VALUES (?, ?, ?)
                   ON DUPLICATE KEY UPDATE
                       config_value = VALUES(config_value),
                       updated_by   = VALUES(updated_by)"#,
                key,
                body.value,
                body.actor
        )
        .execute(pool)
        .await?;

        let row = sqlx::query_as!(
                ConfigRow,
                r#"SELECT config_key, config_value, updated_by, updated_at
                   FROM config WHERE config_key = ?"#,
                key
        )
        .fetch_optional(pool)
        .await?;
        row.ok_or(sqlx::Error::RowNotFound)
}
