pub mod api;
pub mod module;
pub mod test_runner;
pub mod utils;

#[derive(Clone)]
pub struct AppState {
        db_pool: utils::db::Pool,
}

impl AppState {
        pub fn new(db_pool: utils::db::Pool) -> Self {
                Self { db_pool }
        }
}
