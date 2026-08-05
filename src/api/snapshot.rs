use crate::AppState;
use crate::api::{ApiResponse, ApiResponseWithData as resdata, db_error, not_found_response};
use crate::module::snapshot::{controller, model};
use actix_web::{Error, HttpResponse, Responder, get, http::StatusCode, put, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AsOfQuery {
        #[serde(rename = "asOf")]
        pub as_of: Option<String>,
}

#[get("/policy/snapshot")]
pub async fn get_snapshot(
        state: web::Data<AppState>,
        q: web::Query<AsOfQuery>,
) -> Result<impl Responder, Error> {
        let as_of = q.as_of.clone().unwrap_or_default();
        match controller::get(&state.db_pool, &as_of).await {
                Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Ok(None) => Ok(not_found_response("snapshot")),
                Err(e) => Ok(db_error(e)),
        }
}

#[put("/policy/snapshot")]
pub async fn put_snapshot(
        state: web::Data<AppState>,
        form: web::Json<model::PutSnapshot>,
) -> Result<impl Responder, Error> {
        match controller::upsert(&state.db_pool, &form.as_of, &form.payload).await {
                Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("snapshot stored"),
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[get("/feed-snapshot/{key}")]
pub async fn get_feed_snapshot(
        state: web::Data<AppState>,
        path: web::Path<String>,
) -> Result<impl Responder, Error> {
        let key = path.into_inner();
        match controller::get_feed(&state.db_pool, &key).await {
                Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Ok(None) => Ok(not_found_response("feed snapshot")),
                Err(e) => Ok(db_error(e)),
        }
}

#[put("/feed-snapshot/{key}")]
pub async fn put_feed_snapshot(
        state: web::Data<AppState>,
        path: web::Path<String>,
        form: web::Json<model::PutFeedSnapshot>,
) -> Result<impl Responder, Error> {
        let key = path.into_inner();
        match controller::upsert_feed(&state.db_pool, &key, &form.payload).await {
                Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("feed snapshot stored"),
                })),
                Err(e) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(get_snapshot);
        cfg.service(put_snapshot);
        cfg.service(get_feed_snapshot);
        cfg.service(put_feed_snapshot);
}
