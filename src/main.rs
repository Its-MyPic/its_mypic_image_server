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
use utils::env::ENV_CONFIG;


mod endpoints;
mod utils;


#[tokio::main]
async fn main() -> Result<()> {
  utils::env::EnvConfig::load_env().await?;

  let env_config = ENV_CONFIG.get().expect("Failed to load env config.");

  tracing_subscriber::fmt()
    .with_timer(utils::timer::CustomLogTimer)
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
      |response: &Response<Body>, latency: Duration, _span: &Span| {
        tracing::info!(
          " Outgoing  [ {} ]  Took {} ms",
          response.status().as_u16(),
          latency.as_millis()
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
