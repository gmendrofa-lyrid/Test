use crate::AppState;
use crate::api::{
        ApiResponse, ApiResponseWithData as resdata, affected, db_error, not_found_response,
};
use crate::module::stocking::{controller, model};
use actix_web::{Error, HttpResponse, Responder, delete, get, http::StatusCode, put, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CellsQuery {
        pub scope: Option<String>, // "all" returns every cell; default = stocked only
}

#[derive(Deserialize)]
pub struct ItemFilter {
        pub item: Option<String>,
}

#[derive(Deserialize)]
pub struct ClearOverrideQuery {
        pub actor: Option<String>,
}

/// The feed reads this — the detailed materialized policy, one row per item × branch.
#[get("/policy/cells")]
pub async fn list_cells(
        state: web::Data<AppState>,
        q: web::Query<CellsQuery>,
) -> Result<impl Responder, Error> {
        let stocked_only = q.scope.as_deref() != Some("all");
        match controller::get_cells(&state.db_pool, stocked_only).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

/// The refresh job writes this — bulk upsert of ERP-refreshed columns (overrides preserved).
#[put("/policy/cells")]
pub async fn bulk_cells(
        state: web::Data<AppState>,
        form: web::Json<model::BulkCells>,
) -> Result<impl Responder, Error> {
        match controller::bulk_upsert_cells(&state.db_pool, &form.cells).await {
                Ok(result) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("cells refreshed"),
                        data: result,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[get("/policy/params/{item}/{branch}")]
pub async fn get_cell(
        state: web::Data<AppState>,
        path: web::Path<(String, String)>,
) -> Result<impl Responder, Error> {
        let (item, branch) = path.into_inner();
        match controller::get_cell(&state.db_pool, &item, &branch).await {
                Ok(Some(data)) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Ok(None) => Ok(not_found_response("policy cell")),
                Err(e) => Ok(db_error(e)),
        }
}

/// Planner override for ONE field of a cell (sets its `*_ovr` column, logs the change).
#[put("/policy/params/{item}/{branch}")]
pub async fn set_override(
        state: web::Data<AppState>,
        path: web::Path<(String, String)>,
        form: web::Json<model::FieldOverride>,
) -> Result<impl Responder, Error> {
        let (item, branch) = path.into_inner();
        let Some(column) = form.column() else {
                return Ok(HttpResponse::BadRequest().json(ApiResponse {
                        status_code: StatusCode::BAD_REQUEST.as_u16(),
                        message: format!("not an override field: {}", form.field),
                }));
        };
        let value = match form.validated_value() {
                Ok(value) => value,
                Err(message) => {
                        return Ok(HttpResponse::BadRequest().json(ApiResponse {
                                status_code: StatusCode::BAD_REQUEST.as_u16(),
                                message,
                        }));
                }
        };
        match controller::set_field_override(
                &state.db_pool,
                &item,
                &branch,
                &form.field,
                column,
                &value,
                &form.actor,
        )
        .await
        {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("override saved"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

#[delete("/policy/params/{item}/{branch}")]
pub async fn clear_override(
        state: web::Data<AppState>,
        path: web::Path<(String, String)>,
        query: web::Query<ClearOverrideQuery>,
) -> Result<impl Responder, Error> {
        let (item, branch) = path.into_inner();
        let actor = query
                .actor
                .clone()
                .or_else(|| Some(String::from("cockpit")));
        match controller::clear_override(&state.db_pool, &item, &branch, &actor).await {
                Ok(rows) => Ok(affected(rows, "override", "override cleared")),
                Err(e) => Ok(db_error(e)),
        }
}

#[get("/policy/changelog")]
pub async fn changelog(
        state: web::Data<AppState>,
        q: web::Query<ItemFilter>,
) -> Result<impl Responder, Error> {
        let item = q.item.clone().unwrap_or_default();
        match controller::get_changelog(&state.db_pool, &item).await {
                Ok(data) => Ok(HttpResponse::Ok().json(resdata {
                        status_code: StatusCode::OK.as_u16(),
                        message: String::from("success"),
                        data,
                })),
                Err(e) => Ok(db_error(e)),
        }
}

pub fn init(cfg: &mut web::ServiceConfig) {
        cfg.service(list_cells);
        cfg.service(bulk_cells);
        cfg.service(changelog);
        cfg.service(get_cell);
        cfg.service(set_override);
        cfg.service(clear_override);
}
