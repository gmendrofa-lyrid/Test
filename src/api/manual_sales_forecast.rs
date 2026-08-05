use crate::AppState;
use crate::api::{ApiResponseWithData as resdata, db_error};
use crate::module::manual_sales_forecast::controller;
use actix_web::{Error, HttpResponse, Responder, get, http::StatusCode, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoadQuery {
        pub cycle_id: String,
        pub salesperson: Option<String>,
}

#[get("/manual-sales-forecast")]
pub async fn load_manual_sales_forecast(
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

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(load_manual_sales_forecast);
}
