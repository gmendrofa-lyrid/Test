use crate::AppState;
use crate::api::{ApiResponseWithData as resdata, db_error};
use crate::module::config::{controller, model};
use actix_web::{Error, HttpResponse, Responder, get, http, put, web};

#[get("/config")]
pub async fn list_config(state: web::Data<AppState>) -> Result<impl Responder, Error> {
        match controller::get_all(&state.db_pool).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[put("/config/{key}")]
pub async fn set_config(
        state: web::Data<AppState>,
        key: web::Path<String>,
        form: web::Json<model::SetConfig>,
) -> Result<impl Responder, Error> {
        match controller::set(&state.db_pool, &key, &form).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: http::StatusCode::OK.as_u16(),
                        message: String::from("config saved"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(list_config);
        cfg.service(set_config);
}
