use actix_web::{
        App, HttpServer,
        dev::{ServiceRequest, ServiceResponse},
        web,
};
use dotenvy::dotenv;
use snop_cockpit_be::{AppState, api, utils};
use time::{format_description::well_known::Rfc3339, macros::offset};
use tracing::{Span, subscriber::set_global_default};
use tracing_actix_web::RootSpanBuilder;
use tracing_log::LogTracer;
use tracing_subscriber::{
        EnvFilter, Registry,
        fmt::{format::FmtSpan, time::OffsetTime},
        layer::SubscriberExt,
};

pub struct SimpleRootSpanBuilder;

impl RootSpanBuilder for SimpleRootSpanBuilder {
        fn on_request_start(request: &ServiceRequest) -> Span {
                let path = request.path();
                let method = request.method().as_str();
                tracing::info_span!("request", %method, %path)
        }

        fn on_request_end<B>(span: Span, outcome: &Result<ServiceResponse<B>, actix_web::Error>) {
                let status = match outcome {
                        Ok(response) => response.status().as_u16(),
                        Err(e) => e.as_response_error().status_code().as_u16(),
                };
                span.record("status", status);
        }
}

fn init_tracing() {
        LogTracer::init().expect("Failed to set log tracer");
        let env_filter =
                EnvFilter::new("info,actix_web=debug,tracing_actix_web=debug,tower_http=debug");

        let timer = OffsetTime::new(offset!(+7), Rfc3339);

        let subscriber = Registry::default()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer()
                        .with_timer(timer)
                        .with_span_events(FmtSpan::CLOSE)
                        .with_target(true));

        set_global_default(subscriber).expect("Failed to set subscriber");
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
        dotenv().ok();
        init_tracing();

        tracing::info!("starting Server");
        tracing::info!("initializing Database");

        let db_pool = utils::db::Pool::init().expect("unable to connect to database");

        let state = AppState::new(db_pool.clone());

        tracing::info!("binding to port 9093");

        HttpServer::new(move || {
                App::new()
                        .wrap(tracing_actix_web::TracingLogger::<SimpleRootSpanBuilder>::new())
                        .app_data(web::Data::new(state.clone()))
                        // The demand snapshot payload is large (hundreds of KB) — lift the JSON cap.
                        .app_data(web::JsonConfig::default().limit(32 * 1024 * 1024))
                        .configure(api::init)
        })
        .workers(5)
        .bind(("0.0.0.0", 9093))?
        .run()
        .await
}
