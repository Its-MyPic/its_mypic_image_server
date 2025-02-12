use std::io::Cursor;

use axum::{
  extract::Path,
  http::StatusCode,
  response::IntoResponse
};
use image::ImageFormat;
use regex::Regex;
use tokio::{fs, sync::OnceCell};

use crate::utils::{convert::convert_static_image, env::ENV_CONFIG};


static LEGACY_URL_FILE_REGEX: OnceCell<Regex> = OnceCell::const_new();


pub(crate) async fn handler(
  Path(target): Path<String>,
) -> impl IntoResponse {
  let target = target.to_lowercase();

  let regex = match LEGACY_URL_FILE_REGEX.get_or_try_init(
    || async {
      Regex::new(r"(?P<target_file>[ave0-9-]*_[0-9]*)\.(?P<target_format>jpg|jpeg|png|webp)")
    }
  ).await {
    Ok(regex) => regex,
    Err(_) => return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to parse target file."
    ).into_response(),
  };

  let (target_file, target_format) = match regex.captures(&target) {
    Some(result) => {
      let result = result
        .name("target_file")
        .zip(
          result.name("target_format")
        );

      if let Some(result) = result {
        (
          result.0.as_str(),
          result.1.as_str()
        )
      } else {
        return StatusCode::BAD_REQUEST.into_response();
      }
    },
    None => {
      return StatusCode::BAD_REQUEST.into_response();
    },
  };

  let env_config = match ENV_CONFIG.get() {
    Some(env) => env,
    None => return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to read server env."
    ).into_response(),
  };

  let source_file_path = format!(
    "{}/{}.webp",
    env_config.image_source_path,
    target_file
  );

  if let Ok(exists) = fs::try_exists(&source_file_path).await {
    if !exists {
      return (
        StatusCode::NOT_FOUND,
        "Target file not exists."
      ).into_response();
    }
  } else {
    return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Target source file not exists."
    ).into_response();
  }

  let reader = match fs::read(&source_file_path).await {
    Ok(img) => Cursor::new(img),
    Err(_) => return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to read target source file."
    ).into_response(),
  };

  let target_format = match target_format {
    "jpg" | "jpeg" => ImageFormat::Jpeg,
    "png" => ImageFormat::Png,
    "webp" => ImageFormat::WebP,
    _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()
  };

  convert_static_image(reader, target_format).await
}
