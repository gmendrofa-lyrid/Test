use crate::AppState;
use crate::api::{ApiResponse, ApiResponseWithData as resdata, db_error};
use crate::module::consensus::{controller, model};
use crate::module::sales_forecast::controller::WriteError;
use actix_web::{Error, HttpResponse, Responder, get, http::StatusCode, post, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CycleQuery {
        pub cycle_id: String,
}

#[post("/demand-consensus")]
pub async fn save_consensus(
        state: web::Data<AppState>,
        form: web::Json<model::SaveConsensus>,
) -> Result<impl Responder, Error> {
        match controller::save(
                &state.db_pool,
                &form.cycle_id,
                &form.resolved_by,
                &form.picks,
        )
        .await
        {
                Ok(saved) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("consensus picks saved"),
                        data: serde_json::json!({ "saved": saved }),
                })),
                Err(WriteError::NoCycle) => {
                        Ok(conflict(format!("cycle {} does not exist", form.cycle_id)))
                }
                Err(WriteError::Locked(s)) => Ok(conflict(format!(
                        "cycle {} is {}, writes are closed",
                        form.cycle_id, s
                ))),
                Err(WriteError::Db(e)) => Ok(db_error(e)),
        }
}

#[get("/demand-consensus")]
pub async fn load_consensus(
        state: web::Data<AppState>,
        q: web::Query<CycleQuery>,
) -> Result<impl Responder, Error> {
        match controller::load(&state.db_pool, &q.cycle_id).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

fn conflict(message: String) -> HttpResponse {
        HttpResponse::Conflict().json(ApiResponse {
                status_code: StatusCode::CONFLICT.as_u16(),
                message,
        })
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(save_consensus);
        cfg.service(load_consensus);
}
