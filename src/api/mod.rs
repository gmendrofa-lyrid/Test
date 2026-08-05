use crate::AppState;
use actix_web::{HttpResponse, Responder, get, http::StatusCode, web};
use serde::{Deserialize, Serialize};

mod config;
mod consensus;
mod cycle;
mod eoq;
mod manual_sales_forecast;
mod sales_forecast;
mod leadtime;
mod policy;
mod schedule;
mod snapshot;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse {
        pub status_code: u16,
        pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponseWithData<T> {
        pub status_code: u16,
        pub message: String,
        pub data: T,
}

/// Map a `sqlx::Error` to the standard envelope; constraint violations → 409 / 400.
pub fn db_error(e: sqlx::Error) -> HttpResponse {
        use sqlx::error::ErrorKind;
        let (status, message) = match &e {
                sqlx::Error::Database(db) => match db.kind() {
                        ErrorKind::UniqueViolation => (
                                StatusCode::CONFLICT,
                                String::from("resource already exists"),
                        ),
                        ErrorKind::ForeignKeyViolation => (
                                StatusCode::BAD_REQUEST,
                                String::from("referenced resource does not exist"),
                        ),
                        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                },
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        if let sqlx::Error::Database(db) = &e {
                tracing::error!(
                        status = status.as_u16(),
                        code = db.code().as_deref().unwrap_or("-"),
                        "database error: {db}"
                );
        } else if status == StatusCode::INTERNAL_SERVER_ERROR {
                tracing::error!(status = status.as_u16(), error = ?e, "unexpected error");
        } else {
                tracing::warn!(status = status.as_u16(), error = ?e, "request rejected");
        }

        HttpResponse::build(status).json(ApiResponse {
                status_code: status.as_u16(),
                message,
        })
}

/// A 404 with the standard envelope, e.g. `not_found_response("policy override")`.
pub fn not_found_response(entity: &str) -> HttpResponse {
        HttpResponse::NotFound().json(ApiResponse {
                status_code: StatusCode::NOT_FOUND.as_u16(),
                message: format!("{entity} not found"),
        })
}

/// Standard response for an UPDATE/DELETE: 404 when nothing matched, else success.
pub fn affected(rows: u64, entity: &str, message: &str) -> HttpResponse {
        if rows == 0 {
                not_found_response(entity)
        } else {
                HttpResponse::Ok().json(ApiResponseWithData {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from(message),
                        data: "success",
                })
        }
}

/// Health check that also verifies database connectivity.
#[get("/health")]
async fn health_check(state: web::Data<AppState>) -> impl Responder {
        match sqlx::query("SELECT 1").execute(&state.db_pool).await {
                Ok(_) => HttpResponse::Ok().json(ApiResponseWithData {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("ok"),
                        data: serde_json::json!({ "database": "up" }),
                }),
                Err(e) => HttpResponse::ServiceUnavailable().json(ApiResponse {
                        status_code: StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                        message: format!("database unavailable: {e}"),
                }),
        }
}

async fn not_found() -> impl Responder {
        HttpResponse::NotFound().json(ApiResponse {
                status_code: StatusCode::NOT_FOUND.as_u16(),
                message: String::from("not found"),
        })
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(health_check);
        cfg.configure(policy::init);
        cfg.configure(config::init);
        cfg.configure(eoq::init);
        cfg.configure(leadtime::init);
        cfg.configure(schedule::init);
        cfg.configure(snapshot::init);
        cfg.configure(cycle::init);
        cfg.configure(sales_forecast::init);
        cfg.configure(manual_sales_forecast::init);
        cfg.configure(consensus::init);
        cfg.default_service(web::route().to(not_found));
}
