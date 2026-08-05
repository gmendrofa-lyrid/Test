use crate::{AppState, api, utils::db};
use actix_web::{App, http::StatusCode, test, web};
use serde_json::{Value, json};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{ConnectOptions, Executor};
use std::env;
use uuid::Uuid;

macro_rules! ensure {
        ($condition:expr, $message:expr $(, $arg:expr)* $(,)?) => {
                if !$condition {
                        return Err(format!($message $(, $arg)*));
                }
        };
}

/// Native end-to-end API contract test.
///
/// The test deliberately uses a temporary database instead of the configured application schema.
/// That keeps bulk-refresh reconciliation and cleanup from touching real Stocking Policy rows.
pub async fn run() {
        dotenvy::dotenv().ok();

        let suffix = Uuid::new_v4().simple().to_string();
        let database_name = format!("snop_cockpit_test_{}", &suffix[..12]);
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let base_options: MySqlConnectOptions = database_url
                .parse()
                .expect("DATABASE_URL must be a valid MariaDB URL");
        let mut admin = base_options
                .clone()
                .connect()
                .await
                .expect("test database admin connection failed");

        admin.execute(format!("CREATE DATABASE `{database_name}`").as_str())
                .await
                .expect("temporary test database creation failed");

        let test_options = base_options.database(&database_name);
        let migration_pool = MySqlPoolOptions::new()
                .max_connections(2)
                .connect_with(test_options.clone())
                .await
                .expect("temporary test database connection failed");
        sqlx::migrate!("./migrations")
                .run(&migration_pool)
                .await
                .expect("test migrations failed");
        for statement in include_str!("../scripts/seed.sql").split(';') {
                let statement = statement.trim();
                if !statement.is_empty() {
                        migration_pool
                                .execute(statement)
                                .await
                                .expect("test seed failed");
                }
        }

        let app_pool = db::Pool::new(test_options, 4).expect("test application pool failed");
        let outcome: Result<(), String> = {
                let app = test::init_service(
                        App::new()
                                .app_data(web::Data::new(AppState::new(app_pool.clone())))
                                .app_data(web::JsonConfig::default().limit(32 * 1024 * 1024))
                                .configure(api::init),
                )
                .await;

                async {
                        // Health
                        let response =
                                test::call_service(&app, test::TestRequest::get().uri("/health").to_request())
                                        .await;
                        ensure!(response.status() == StatusCode::OK, "health returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["database"] == "up", "health did not report database up: {body}");

                        // Config defaults + write
                        let response =
                                test::call_service(&app, test::TestRequest::get().uri("/config").to_request())
                                        .await;
                        ensure!(response.status() == StatusCode::OK, "config GET returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        let config = body["data"].as_array().ok_or("config data was not an array")?;
                        ensure!(
                                config.iter().any(|row| {
                                        row["config_key"] == "target_dio_days" && row["config_value"] == "100"
                                }),
                                "target_dio_days default missing: {body}"
                        );
                        ensure!(
                                config.iter().any(|row| {
                                        row["config_key"] == "default_service_level"
                                                && row["config_value"] == "0.95"
                                }),
                                "default_service_level default missing: {body}"
                        );
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/config/rust_test_config")
                                        .set_json(json!({"value":"120","actor":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "config PUT returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["config_value"] == "120", "config PUT did not persist: {body}");

                        // Durable schedule: future run is not claimable, a due run is, success returns
                        // to midnight WIB, and failure records a one-hour retry.
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/job-schedules/stocking_policy_refresh")
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "schedule GET returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["interval_months"] == 6, "schedule reconciliation interval was not six months: {body}");
                        ensure!(body["data"]["interval_hours"] == 24, "schedule operational interval was not daily: {body}");
                        ensure!(
                                body["data"]["next_run_at"]
                                        .as_str()
                                        .is_some_and(|value| value.contains("T17:00:00")),
                                "schedule was not midnight WIB: {body}"
                        );
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/job-schedules/stocking_policy_refresh/claim")
                                        .set_json(json!({"owner":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["claimed"] == false, "future schedule was unexpectedly claimed: {body}");

                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/job-schedules/stocking_policy_refresh")
                                        .set_json(json!({
                                                "next_run_at":"2000-01-01T17:00:00Z",
                                                "actor":"rust-test"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "schedule reschedule returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/job-schedules/stocking_policy_refresh/claim")
                                        .set_json(json!({"owner":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["claimed"] == true, "due schedule was not claimed: {body}");
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/job-schedules/stocking_policy_refresh/complete")
                                        .set_json(json!({"owner":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "schedule complete returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["status"] == "idle", "completed schedule was not idle: {body}");
                        ensure!(
                                body["data"]["next_run_at"]
                                        .as_str()
                                        .is_some_and(|value| value.contains("T17:00:00")),
                                "completed schedule drifted from midnight WIB: {body}"
                        );

                        // Stocking Policy bulk materialization + per-field overrides.
                        let initial_cells = json!({
                                "cells": [
                                        {
                                                "item_code":"1196","branch":"Jakarta","item_name":"TEXAPON",
                                                "item_group":"IONIC","primary_item_group":"","uom":"Kg",
                                                "stocked":true,"cls":"A·X","uc":33000,"oh_qty":100,
                                                "oh_value":3300000,"lead_time":60,"dd":10,"md":300,
                                                "aq":3650,"sd":4,"obs":12
                                        },
                                        {
                                                "item_code":"1196","branch":"Semarang","item_name":"TEXAPON",
                                                "item_group":"IONIC","primary_item_group":"","uom":"Kg",
                                                "stocked":true,"cls":"A·X","uc":26000,"oh_qty":50,
                                                "oh_value":1300000,"lead_time":60,"dd":3,"md":90,
                                                "aq":1095,"sd":2,"obs":12
                                        }
                                ]
                        });
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/cells")
                                        .set_json(&initial_cells)
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "policy bulk PUT returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["upserted"] == 2, "policy bulk PUT did not upsert two cells: {body}");

                        let response = test::call_service(
                                &app,
                                test::TestRequest::get().uri("/policy/cells?scope=all").to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"].as_array().is_some_and(|rows| rows.len() == 2), "policy GET did not return two cells: {body}");

                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/params/1196/Jakarta")
                                        .set_json(json!({"field":"service_level","value":0.99,"actor":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["sl_ovr"] == 0.99, "service-level override failed: {body}");
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/params/1196/Jakarta")
                                        .set_json(json!({"field":"lead_time","value":42,"actor":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["lt_ovr"] == 42.0, "lead-time override failed: {body}");
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/params/1196/Jakarta")
                                        .set_json(json!({"field":"stocked","value":false,"actor":"rust-test"}))
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["st_ovr"] == false, "stocked override failed: {body}");

                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/params/1196/Jakarta")
                                        .set_json(json!({"field":"bogus","value":1}))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::BAD_REQUEST, "unknown override returned {}", response.status());

                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/policy/changelog?item=1196")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["data"].as_array().is_some_and(|rows| rows.iter().any(|row| {
                                        row["field"] == "lead_time" && row["to_value"] == "42"
                                })),
                                "lead-time change log missing: {body}"
                        );

                        let refreshed_cell = json!({
                                "cells":[{
                                        "item_code":"1196","branch":"Jakarta","item_name":"TEXAPON",
                                        "item_group":"IONIC","primary_item_group":"","uom":"Kg",
                                        "stocked":true,"cls":"A·X","uc":34000,"oh_qty":120,
                                        "oh_value":4080000,"lead_time":60,"dd":11,"md":330,
                                        "aq":4015,"sd":5,"obs":12
                                }]
                        });
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/cells")
                                        .set_json(&refreshed_cell)
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "policy refresh returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/policy/params/1196/Jakarta")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["dd"] == 11.0, "ERP demand was not refreshed: {body}");
                        ensure!(body["data"]["sl_ovr"] == 0.99, "service-level override was lost: {body}");
                        ensure!(body["data"]["lt_ovr"] == 42.0, "lead-time override was lost: {body}");

                        let response = test::call_service(
                                &app,
                                test::TestRequest::delete()
                                        .uri("/policy/params/1196/Jakarta?actor=rust-test")
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "override DELETE returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/policy/params/1196/Jakarta")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["sl_ovr"].is_null(), "service-level override was not cleared: {body}");
                        ensure!(body["data"]["lt_ovr"].is_null(), "lead-time override was not cleared: {body}");

                        // EOQ
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/eoq")
                                        .set_json(json!({
                                                "item_group":"",
                                                "ordering_cost":250000,
                                                "holding_pct":0.22,
                                                "active":true,
                                                "actor":"rust-test"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "EOQ PUT returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["active"] == true, "EOQ row was not activated: {body}");
                        let response =
                                test::call_service(&app, test::TestRequest::get().uri("/eoq").to_request())
                                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"].as_array().is_some_and(|rows| rows.len() == 1), "EOQ upsert created duplicates: {body}");

                        // Legacy demand snapshot
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/policy/snapshot?asOf=2000-01-01")
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::NOT_FOUND, "missing snapshot returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::put()
                                        .uri("/policy/snapshot")
                                        .set_json(json!({
                                                "as_of":"2000-01-01",
                                                "payload":"{\"stocked\":[\"X\"]}"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "snapshot PUT returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/policy/snapshot?asOf=2000-01-01")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["data"]["payload"]
                                        .as_str()
                                        .is_some_and(|payload| payload.contains("stocked")),
                                "snapshot payload did not round-trip: {body}"
                        );

                        // Consensus freeze integrity: reject empty/incomplete plans without changing
                        // either table, then atomically freeze a complete plan and make retries no-op.
                        let target_months = "[\"2099-01\",\"2099-02\"]";
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/ensure")
                                        .set_json(json!({
                                                "cycle_id":"freeze-empty",
                                                "target_months":target_months
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "empty cycle ensure returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/freeze")
                                        .set_json(json!({
                                                "cycle_id":"freeze-empty",
                                                "frozen_by":"rust-test"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::CONFLICT, "empty freeze returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["message"].as_str().is_some_and(|message| message.contains("no consensus rows")),
                                "empty freeze returned the wrong error: {body}"
                        );
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/cycle/freeze-empty")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["status"] == "open", "empty freeze changed cycle state: {body}");

                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/ensure")
                                        .set_json(json!({
                                                "cycle_id":"freeze-incomplete",
                                                "target_months":target_months
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "incomplete cycle ensure returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/demand-consensus")
                                        .set_json(json!({
                                                "cycle_id":"freeze-incomplete",
                                                "resolved_by":"rust-test",
                                                "picks":[{
                                                        "customer":"CUST-1",
                                                        "item_code":"ITEM-1",
                                                        "branch":"Jakarta",
                                                        "ym":"2099-01",
                                                        "selected_source":"Consensus",
                                                        "selected_qty":10.0,
                                                        "reason":null
                                                }]
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "incomplete consensus save returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/freeze")
                                        .set_json(json!({
                                                "cycle_id":"freeze-incomplete",
                                                "frozen_by":"rust-test"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::CONFLICT, "incomplete freeze returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["message"].as_str().is_some_and(|message| message.contains("2099-02")),
                                "incomplete freeze did not identify the missing month: {body}"
                        );
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/cycle/freeze-incomplete")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["status"] == "open", "incomplete freeze changed cycle state: {body}");
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/demand-consensus?cycle_id=freeze-incomplete")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["data"].as_array().is_some_and(|rows| {
                                        rows.len() == 1 && rows.iter().all(|row| row["is_frozen"] == false)
                                }),
                                "incomplete freeze changed consensus rows: {body}"
                        );

                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/ensure")
                                        .set_json(json!({
                                                "cycle_id":"freeze-complete",
                                                "target_months":target_months
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "complete cycle ensure returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/demand-consensus")
                                        .set_json(json!({
                                                "cycle_id":"freeze-complete",
                                                "resolved_by":"rust-test",
                                                "picks":[
                                                        {
                                                                "customer":"CUST-1",
                                                                "item_code":"ITEM-1",
                                                                "branch":"Jakarta",
                                                                "ym":"2099-01",
                                                                "selected_source":"Consensus",
                                                                "selected_qty":10.0,
                                                                "reason":null
                                                        },
                                                        {
                                                                "customer":"CUST-1",
                                                                "item_code":"ITEM-1",
                                                                "branch":"Jakarta",
                                                                "ym":"2099-02",
                                                                "selected_source":"Sales",
                                                                "selected_qty":12.0,
                                                                "reason":null
                                                        }
                                                ]
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "complete consensus save returned {}", response.status());
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/freeze")
                                        .set_json(json!({
                                                "cycle_id":"freeze-complete",
                                                "frozen_by":"rust-test"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "complete freeze returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["locked"] == 2, "complete freeze did not lock both rows: {body}");
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/cycle/freeze-complete")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["status"] == "frozen", "complete freeze did not freeze cycle: {body}");
                        let first_frozen_at = body["data"]["frozen_at"].clone();
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/demand-consensus?cycle_id=freeze-complete")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["data"].as_array().is_some_and(|rows| {
                                        rows.len() == 2 && rows.iter().all(|row| row["is_frozen"] == true)
                                }),
                                "complete freeze did not freeze every consensus row: {body}"
                        );
                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/demand-consensus")
                                        .set_json(json!({
                                                "cycle_id":"freeze-complete",
                                                "resolved_by":"late-writer",
                                                "picks":[{
                                                        "customer":"CUST-1",
                                                        "item_code":"ITEM-1",
                                                        "branch":"Jakarta",
                                                        "ym":"2099-01",
                                                        "selected_source":"Manual",
                                                        "selected_qty":999.0,
                                                        "reason":"late update"
                                                }]
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(
                                response.status() == StatusCode::CONFLICT,
                                "post-freeze consensus save returned {}",
                                response.status()
                        );

                        let response = test::call_service(
                                &app,
                                test::TestRequest::post()
                                        .uri("/cycle/freeze")
                                        .set_json(json!({
                                                "cycle_id":"freeze-complete",
                                                "frozen_by":"retry-user"
                                        }))
                                        .to_request(),
                        )
                        .await;
                        ensure!(response.status() == StatusCode::OK, "repeated freeze returned {}", response.status());
                        let body: Value = test::read_body_json(response).await;
                        ensure!(body["data"]["locked"] == 0, "repeated freeze was not a no-op: {body}");
                        let response = test::call_service(
                                &app,
                                test::TestRequest::get()
                                        .uri("/cycle/freeze-complete")
                                        .to_request(),
                        )
                        .await;
                        let body: Value = test::read_body_json(response).await;
                        ensure!(
                                body["data"]["frozen_at"] == first_frozen_at,
                                "repeated freeze changed the original frozen timestamp: {body}"
                        );

                        Ok(())
                }
                .await
        };

        drop(app_pool);
        migration_pool.close().await;
        admin.execute(format!("DROP DATABASE `{database_name}`").as_str())
                .await
                .expect("temporary test database cleanup failed");

        if let Err(message) = outcome {
                panic!("native API contract failed: {message}");
        }
}

#[cfg(test)]
mod tests {
        #[actix_web::test]
        async fn full_api_contract_uses_an_isolated_database() {
                super::run().await;
        }
}
