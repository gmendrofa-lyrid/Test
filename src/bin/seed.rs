//! Sample-data seeder. Applies `scripts/seed.sql` statement-by-statement.
//!
//! Run schema migrations first (`sqlx migrate run`), then: `cargo run --bin seed`.

use sqlx::mysql::MySqlConnectOptions;
use sqlx::{ConnectOptions, Executor};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let sql = include_str!("../../scripts/seed.sql");
        let options: MySqlConnectOptions = env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set")
                .parse()?;
        let mut conn = options.connect().await?;

        let mut count = 0;
        for stmt in sql.split(';') {
                let stmt = stmt.trim();
                if stmt.is_empty() {
                        continue;
                }
                conn.execute(stmt).await?;
                count += 1;
        }

        println!("seed complete: {count} statements executed");
        Ok(())
}
