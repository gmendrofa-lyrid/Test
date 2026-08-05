use crate::AppState;
use crate::api::{ApiResponseWithData as resdata, db_error};
use crate::module::eoq::{controller, model};
use actix_web::{Error, HttpResponse, Responder, get, http, put, web};

#[get("/eoq")]
pub async fn list_eoq(state: web::Data<AppState>) -> Result<impl Responder, Error> {
        match controller::get_all(&state.db_pool).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[put("/eoq")]
pub async fn upsert_eoq(
        state: web::Data<AppState>,
        form: web::Json<model::UpsertEoq>,
) -> Result<impl Responder, Error> {
        match controller::upsert(&state.db_pool, &form).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: String::from("eoq params saved"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(list_eoq);
        cfg.service(upsert_eoq);
}
