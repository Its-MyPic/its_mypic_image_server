use tokio::sync::OnceCell;
use anyhow::Result;

pub(crate) static ENV_CONFIG: OnceCell<EnvConfig> = OnceCell::const_new();

pub(crate) struct EnvConfig {
  pub(crate) server_ip: String,
  pub(crate) server_port: String,
  pub(crate) image_source_path: String
}

impl EnvConfig {
  fn new() -> Result<Self> {
    Ok(
      Self {
        server_ip: std::env::var("SERVER_IP")?,
        server_port: std::env::var("SERVER_PORT")?,
        image_source_path: std::env::var("IMAGE_SOURCE_PATH")?
      }
    )
  }

  pub(crate) async fn load_env() -> Result<&'static Self> {
    match dotenvy::from_filename_override(".env") {
      Ok(_) => {},
      Err(_) => {},
    };

    ENV_CONFIG.get_or_try_init(
      || async {
        Self::new()
      }
    ).await
  }
}
