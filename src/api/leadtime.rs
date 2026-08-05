use crate::AppState;
use crate::api::{ApiResponse, ApiResponseWithData as resdata, affected, db_error};
use crate::module::leadtime::{controller, model};
use actix_web::{Error, HttpResponse, Responder, delete, get, http::StatusCode, put, web};

/// PM-maintained lead-time defaults (per principal x PIG x item_group x leg), used while the real
/// PO->GR export is missing. Read by the Lead-Time Settings screen.
#[get("/lead-time-defaults")]
pub async fn list_defaults(state: web::Data<AppState>) -> Result<impl Responder, Error> {
        match controller::get_all(&state.db_pool).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[put("/lead-time-defaults")]
pub async fn upsert_default(
        state: web::Data<AppState>,
        form: web::Json<model::UpsertPmLeadTime>,
) -> Result<impl Responder, Error> {
        if !form.valid_leg() {
                return Ok(HttpResponse::BadRequest().json(ApiResponse {
                        status_code: StatusCode::BAD_REQUEST.as_u16(),
                        message: format!("unknown lead-time leg: {}", form.leg),
                }));
        }
        match controller::upsert(&state.db_pool, &form).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("lead-time default saved"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[delete("/lead-time-defaults/{id}")]
pub async fn delete_default(
        state: web::Data<AppState>,
        path: web::Path<String>,
) -> Result<impl Responder, Error> {
        let id = path.into_inner();
        match controller::delete(&state.db_pool, &id).await {
                Ok(rows) => Ok(affected(rows, "lead-time default", "lead-time default deleted")),
                Err(e) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(list_defaults);
        cfg.service(upsert_default);
        cfg.service(delete_default);
}
