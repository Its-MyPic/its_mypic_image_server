use axum::{
  http::Method, routing::get, Router
};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use anyhow::Result;
use utils::env::ENV_CONFIG;


mod endpoints;
mod utils;


#[tokio::main]
async fn main() -> Result<()> {
  utils::env::EnvConfig::load_env().await?;

  let env_config = ENV_CONFIG.get().expect("Failed to load env config.");

  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

  let cors = CorsLayer::new()
    .allow_methods([Method::GET])
    .allow_origin(Any);

  let app = Router::new()
    .route("/images/{target}", get(endpoints::legacy_image::handler))
    .route("/images/{season}/{episode}/{target}", get(endpoints::image::handler))
    .layer(TraceLayer::new_for_http())
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
