use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

/// One detailed Stocking Policy cell (item × branch): ERP-refreshed facts + planner overrides.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct StockingCell {
        pub item_code: String,
        pub branch: String,
        pub item_name: Option<String>,
        pub item_group: Option<String>,
        pub primary_item_group: Option<String>,
        pub uom: Option<String>,
        pub stocked: bool,
        pub cls: Option<String>,
        pub uc: Option<f64>,
        pub oh_qty: f64,
        pub oh_value: f64,
        pub lead_time: Option<f64>,
        pub dd: f64,
        pub md: f64,
        pub aq: f64,
        pub sd: f64,
        pub obs: i32,
        pub refreshed_at: Option<DateTime<Utc>>,
        pub sl_ovr: Option<f64>,
        pub ot_ovr: Option<String>,
        pub moq_ovr: Option<f64>,
        pub inc_ovr: Option<f64>,
        pub sigma_lt_ovr: Option<f64>,
        pub st_ovr: Option<bool>,
        pub lt_ovr: Option<f64>,
        pub override_updated_by: Option<String>,
        pub override_updated_at: Option<DateTime<Utc>>,
}

/// ERP-refreshed columns for one cell (bulk write from the refresh job; overrides untouched).
#[derive(Debug, Serialize, Deserialize)]
pub struct CellUpsert {
        pub item_code: String,
        pub branch: String,
        pub item_name: Option<String>,
        pub item_group: Option<String>,
        pub primary_item_group: Option<String>,
        pub uom: Option<String>,
        pub stocked: bool,
        pub cls: Option<String>,
        pub uc: Option<f64>,
        pub oh_qty: f64,
        pub oh_value: f64,
        pub lead_time: Option<f64>,
        pub dd: f64,
        pub md: f64,
        pub aq: f64,
        pub sd: f64,
        pub obs: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkCells {
        pub cells: Vec<CellUpsert>,
}

/// PUT body for a planner override — ONE field at a time (matches the per-cell edit in the table),
/// so editing e.g. lead time doesn't pin the other columns. `value: null` clears that override.
#[derive(Debug, Deserialize)]
pub struct FieldOverride {
        /// service_level | order_type | moq | increment | sigma_lt | stocked | lead_time
        pub field: String,
        pub value: Option<serde_json::Value>,
        pub actor: Option<String>,
}

impl FieldOverride {
        /// Map the canonical field name to its `*_ovr` column, or None if not a valid override field.
        pub fn column(&self) -> Option<&'static str> {
                match self.field.as_str() {
                        "service_level" => Some("sl_ovr"),
                        "order_type" => Some("ot_ovr"),
                        "moq" => Some("moq_ovr"),
                        "increment" => Some("inc_ovr"),
                        "sigma_lt" => Some("sigma_lt_ovr"),
                        "stocked" => Some("st_ovr"),
                        "lead_time" => Some("lt_ovr"),
                        _ => None,
                }
        }

        /// Validate and normalize an override before it reaches SQL. JSON `null` deliberately
        /// remains a supported way to clear one field without disturbing the others.
        pub fn validated_value(&self) -> Result<ValidatedOverrideValue, String> {
                let Some(value) = self.value.as_ref() else {
                        return Ok(ValidatedOverrideValue::Clear);
                };

                match self.field.as_str() {
                        "service_level" => {
                                let n = finite_number(value, "service_level")?;
                                if !(0.50..=0.9999).contains(&n) {
                                        return Err(String::from(
                                                "service_level must be between 0.50 and 0.9999",
                                        ));
                                }
                                Ok(ValidatedOverrideValue::Number(n))
                        }
                        "lead_time" | "sigma_lt" | "moq" | "increment" => {
                                let n = finite_number(value, &self.field)?;
                                if n < 0.0 {
                                        return Err(format!(
                                                "{} must be greater than or equal to 0",
                                                self.field
                                        ));
                                }
                                Ok(ValidatedOverrideValue::Number(n))
                        }
                        "stocked" => value
                                .as_bool()
                                .map(ValidatedOverrideValue::Bool)
                                .ok_or_else(|| String::from("stocked must be a boolean")),
                        "order_type" => {
                                let order_type = value
                                        .as_str()
                                        .ok_or_else(|| String::from("order_type must be a string"))?
                                        .trim()
                                        .to_ascii_lowercase();
                                if !matches!(
                                        order_type.as_str(),
                                        "planned" | "buy" | "transfer" | "work_order"
                                ) {
                                        return Err(String::from(
                                                "order_type must be one of: planned, buy, transfer, work_order",
                                        ));
                                }
                                Ok(ValidatedOverrideValue::Text(order_type))
                        }
                        _ => Err(format!("not an override field: {}", self.field)),
                }
        }
}

fn finite_number(value: &serde_json::Value, field: &str) -> Result<f64, String> {
        let n = value
                .as_f64()
                .ok_or_else(|| format!("{field} must be a number"))?;
        if !n.is_finite() {
                return Err(format!("{field} must be finite"));
        }
        Ok(n)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedOverrideValue {
        Clear,
        Number(f64),
        Text(String),
        Bool(bool),
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChangeLogEntry {
        pub id: String,
        pub item_code: String,
        pub branch: String,
        pub field: String,
        pub from_value: Option<String>,
        pub to_value: Option<String>,
        pub actor: Option<String>,
        pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
        use super::{FieldOverride, ValidatedOverrideValue};
        use serde_json::json;

        fn field(field: &str, value: Option<serde_json::Value>) -> FieldOverride {
                FieldOverride {
                        field: field.to_owned(),
                        value,
                        actor: None,
                }
        }

        #[test]
        fn null_clears_every_supported_field() {
                for name in [
                        "service_level",
                        "order_type",
                        "moq",
                        "increment",
                        "sigma_lt",
                        "stocked",
                        "lead_time",
                ] {
                        assert_eq!(
                                field(name, None).validated_value(),
                                Ok(ValidatedOverrideValue::Clear)
                        );
                }
        }

        #[test]
        fn validates_numeric_ranges_and_types() {
                assert!(field("service_level", Some(json!(0.50)))
                        .validated_value()
                        .is_ok());
                assert!(field("service_level", Some(json!(0.9999)))
                        .validated_value()
                        .is_ok());
                assert!(field("service_level", Some(json!(0.49)))
                        .validated_value()
                        .is_err());
                assert!(field("service_level", Some(json!(1)))
                        .validated_value()
                        .is_err());
                assert!(field("lead_time", Some(json!(0))).validated_value().is_ok());
                assert!(field("sigma_lt", Some(json!(-1)))
                        .validated_value()
                        .is_err());
                assert!(field("moq", Some(json!("10"))).validated_value().is_err());
        }

        #[test]
        fn validates_and_normalizes_categorical_values() {
                assert_eq!(
                        field("order_type", Some(json!(" Work_Order "))).validated_value(),
                        Ok(ValidatedOverrideValue::Text(String::from("work_order")))
                );
                assert!(field("order_type", Some(json!("make")))
                        .validated_value()
                        .is_err());
                assert!(field("stocked", Some(json!(true)))
                        .validated_value()
                        .is_ok());
                assert!(field("stocked", Some(json!(1))).validated_value().is_err());
        }
}
