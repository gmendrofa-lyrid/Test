use crate::AppState;
use crate::api::{ApiResponse, ApiResponseWithData as resdata, db_error, not_found_response};
use crate::module::cycle::{controller, controller::FreezeError, model};
use actix_web::{Error, HttpResponse, Responder, get, http::StatusCode, post, web};

#[get("/cycle/{cycle_id}")]
pub async fn get_cycle(
        state: web::Data<AppState>,
        path: web::Path<String>,
) -> Result<impl Responder, Error> {
        let cycle_id = path.into_inner();
        match controller::get(&state.db_pool, &cycle_id).await {
                Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Ok(None) => Ok(not_found_response("cycle")),
                Err(e) => Ok(db_error(e)),
        }
}

#[post("/cycle/ensure")]
pub async fn ensure_cycle(
        state: web::Data<AppState>,
        form: web::Json<model::EnsureCycle>,
) -> Result<impl Responder, Error> {
        match controller::ensure(
                &state.db_pool,
                &form.cycle_id,
                form.label.as_deref(),
                form.target_months.as_deref(),
        )
        .await
        {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("cycle ready"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[post("/cycle/freeze")]
pub async fn freeze_cycle(
        state: web::Data<AppState>,
        form: web::Json<model::FreezeCycle>,
) -> Result<impl Responder, Error> {
        match controller::freeze(&state.db_pool, &form.cycle_id, &form.frozen_by).await {
                Ok(locked) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("cycle frozen"),
                        data: serde_json::json!({ "frozen": form.cycle_id, "locked": locked }),
                })),
                Err(FreezeError::NotFound) => Ok(not_found_response("cycle")),
                Err(FreezeError::Closed) => Ok(HttpResponse::Conflict().json(ApiResponse {
                        status_code: StatusCode::CONFLICT.as_u16(),
                        message: format!("cycle {} is closed", form.cycle_id),
                })),
                Err(FreezeError::Empty) => Ok(HttpResponse::Conflict().json(ApiResponse {
                        status_code: StatusCode::CONFLICT.as_u16(),
                        message: format!("cycle {} has no consensus rows", form.cycle_id),
                })),
                Err(FreezeError::Incomplete { missing_months }) => {
                        Ok(HttpResponse::Conflict().json(ApiResponse {
                                status_code: StatusCode::CONFLICT.as_u16(),
                                message: format!(
                                        "cycle {} consensus is incomplete; missing target months: {}",
                                        form.cycle_id,
                                        missing_months.join(", ")
                                ),
                        }))
                }
                Err(FreezeError::InvalidTargetMonths) => {
                        Ok(HttpResponse::Conflict().json(ApiResponse {
                                status_code: StatusCode::CONFLICT.as_u16(),
                                message: format!(
                                        "cycle {} has no valid target months",
                                        form.cycle_id
                                ),
                        }))
                }
                Err(FreezeError::Db(e)) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(ensure_cycle);
        cfg.service(freeze_cycle);
        cfg.service(get_cycle);
}
