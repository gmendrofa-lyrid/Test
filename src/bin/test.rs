#[actix_web::main]
async fn main() {
        println!("running native S&OP API contract tests");
        snop_cockpit_be::test_runner::run().await;
        println!("native S&OP API contract tests passed");
}
