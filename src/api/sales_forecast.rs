use crate::AppState;
use crate::api::{ApiResponse, ApiResponseWithData as resdata, db_error};
use crate::module::sales_forecast::{controller, controller::WriteError, model};
use actix_web::{Error, HttpResponse, Responder, get, http::StatusCode, post, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoadQuery {
        pub cycle_id: String,
        pub salesperson: Option<String>,
}

#[post("/sales-forecast")]
pub async fn save_sales_forecast(
        state: web::Data<AppState>,
        form: web::Json<model::SaveSalesForecast>,
) -> Result<impl Responder, Error> {
        match controller::save(
                &state.db_pool,
                &form.cycle_id,
                &form.updated_by,
                &form.entries,
        )
        .await
        {
                Ok(saved) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("sales forecast saved"),
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

#[get("/sales-forecast")]
pub async fn load_sales_forecast(
        state: web::Data<AppState>,
        q: web::Query<LoadQuery>,
) -> Result<impl Responder, Error> {
        let salesperson = q.salesperson.clone().unwrap_or_default();
        match controller::load(&state.db_pool, &q.cycle_id, &salesperson).await {
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
        cfg.service(save_sales_forecast);
        cfg.service(load_sales_forecast);
}
