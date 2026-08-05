use crate::AppState;
use crate::api::{ApiResponse, ApiResponseWithData as resdata, db_error, not_found_response};
use crate::module::schedule::{controller, model};
use actix_web::{Error, HttpResponse, Responder, get, http, post, put, web};

fn bad_request(message: impl Into<String>) -> HttpResponse {
        HttpResponse::BadRequest().json(ApiResponse {
                status_code: http::StatusCode::BAD_REQUEST.as_u16(),
                message: message.into(),
        })
}

fn lost_lease() -> HttpResponse {
        HttpResponse::Conflict().json(ApiResponse {
                status_code: http::StatusCode::CONFLICT.as_u16(),
                message: String::from("job lease is missing, expired, or owned by another worker"),
        })
}

#[get("/job-schedules/{key}")]
pub async fn get_schedule(
        state: web::Data<AppState>,
        key: web::Path<String>,
) -> Result<impl Responder, Error> {
        match controller::get(&state.db_pool, &key).await {
                Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Ok(None) => Ok(not_found_response("job schedule")),
                Err(e) => Ok(db_error(e)),
        }
}

#[put("/job-schedules/{key}")]
pub async fn update_schedule(
        state: web::Data<AppState>,
        key: web::Path<String>,
        form: web::Json<model::UpdateSchedule>,
) -> Result<impl Responder, Error> {
        if matches!(form.interval_months, Some(0 | 121..=u16::MAX)) {
                return Ok(bad_request("interval_months must be between 1 and 120"));
        }
        match controller::update(&state.db_pool, &key, &form).await {
                Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: String::from("job schedule saved"),
                        data,
                })),
                Ok(None) => Ok(not_found_response("job schedule")),
                Err(e) => Ok(db_error(e)),
        }
}

#[post("/job-schedules/{key}/claim")]
pub async fn claim_job(
        state: web::Data<AppState>,
        key: web::Path<String>,
        form: web::Json<model::ClaimJob>,
) -> Result<impl Responder, Error> {
        let owner = form.owner.trim();
        if owner.is_empty() || owner.len() > 128 {
                return Ok(bad_request("owner must contain 1 to 128 characters"));
        }
        match controller::claim(&state.db_pool, &key, owner).await {
                Ok(data) if data.schedule.is_none() => Ok(not_found_response("job schedule")),
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: if data.claimed {
                                String::from("job claimed")
                        } else {
                                String::from("job is not due or already leased")
                        },
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[post("/job-schedules/{key}/complete")]
pub async fn complete_job(
        state: web::Data<AppState>,
        key: web::Path<String>,
        form: web::Json<model::FinishJob>,
) -> Result<impl Responder, Error> {
        let owner = form.owner.trim();
        if owner.is_empty() || owner.len() > 128 {
                return Ok(bad_request("owner must contain 1 to 128 characters"));
        }
        match controller::complete(&state.db_pool, &key, owner).await {
                Ok(1) => match controller::get(&state.db_pool, &key).await {
                        Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                                status_code: http::StatusCode::OK.as_u16(),
                                message: String::from("job completed"),
                                data,
                        })),
                        Ok(None) => Ok(not_found_response("job schedule")),
                        Err(e) => Ok(db_error(e)),
                },
                Ok(_) => Ok(lost_lease()),
                Err(e) => Ok(db_error(e)),
        }
}

#[post("/job-schedules/{key}/fail")]
pub async fn fail_job(
        state: web::Data<AppState>,
        key: web::Path<String>,
        form: web::Json<model::FinishJob>,
) -> Result<impl Responder, Error> {
        let owner = form.owner.trim();
        if owner.is_empty() || owner.len() > 128 {
                return Ok(bad_request("owner must contain 1 to 128 characters"));
        }
        let error = form
                .error
                .as_deref()
                .unwrap_or("stocking-policy refresh failed");
        let error = error.chars().take(4000).collect::<String>();
        match controller::fail(&state.db_pool, &key, owner, &error).await {
                Ok(1) => match controller::get(&state.db_pool, &key).await {
                        Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                                status_code: http::StatusCode::OK.as_u16(),
                                message: String::from("job failure recorded"),
                                data,
                        })),
                        Ok(None) => Ok(not_found_response("job schedule")),
                        Err(e) => Ok(db_error(e)),
                },
                Ok(_) => Ok(lost_lease()),
                Err(e) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(get_schedule);
        cfg.service(update_schedule);
        cfg.service(claim_job);
        cfg.service(complete_job);
        cfg.service(fail_job);
}
