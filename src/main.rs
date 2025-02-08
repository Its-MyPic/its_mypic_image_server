use std::time::Duration;

use axum::{
  body::Body,
  http::{Method, Request, Response},
  routing::get,
  Router
};
use tower_http::{
  cors::{Any, CorsLayer},
  trace::TraceLayer
};
use anyhow::Result;
use tracing::{Level, Span};
use chrono::{DateTime, Local};
use tracing_subscriber::fmt::time::FormatTime;
use utils::env::ENV_CONFIG;


mod endpoints;
mod utils;


struct CustomTimer;

impl FormatTime for CustomTimer {
  fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
    let current_local: DateTime<Local> = Local::now();
    write!(w, "{}", current_local.format("%F %T%.3f"))
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  utils::env::EnvConfig::load_env().await?;

  let env_config = ENV_CONFIG.get().expect("Failed to load env config.");

  tracing_subscriber::fmt()
    .with_timer(CustomTimer)
    .with_target(false)
    .with_max_level(Level::INFO)
    .init();

  let trace = TraceLayer::new_for_http()
    .on_request(
      |request: &Request<Body>, _span: &Span| {
        tracing::info!(
          " Incoming  [ {} ]  {}",
          request.method(),
          request.uri().path()
        );
      }
    )
    .on_response(
      |_response: &Response<Body>, _latency: Duration, _span: &Span| {
        tracing::info!(
          " Outgoing  [ {} ]  Took {} ms",
          _response.status().as_u16(),
          _latency.as_millis()
        );
      }
    );

  let cors = CorsLayer::new()
    .allow_methods([Method::GET])
    .allow_origin(Any);

  let app = Router::new()
    .route("/images/{target}", get(endpoints::legacy_image::handler))
    .route("/images/{season}/{episode}/{target}", get(endpoints::image::handler))
    .layer(trace)
    .layer(cors);

  let listener = tokio::net::TcpListener::bind(
    format!(
      "{}:{}",
      env_config.server_ip,
      env_config.server_port
    )
  ).await?;

  axum::serve(listener, app).await?;

  Ok(())
}
