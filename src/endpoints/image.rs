use std::{io::Cursor, sync::Arc};

use axum::{
  body::Body,
  extract::{Path, State},
  http::{Response, StatusCode},
  response::IntoResponse
};
use image::ImageFormat;
use tokio::fs;

use crate::{
  utils::{
    convert::{
      convert_animated_image,
      convert_static_image
    },
    env::{
      EnvConfig,
      ENV_CONFIG
    }
  },
  Scheduler
};


pub(crate) async fn handler(
  Path((season, episode, target)): Path<(String, String, String)>,
  State(scheduler): State<Arc<Scheduler>>,
) -> impl IntoResponse {
  let season = season.to_lowercase();
  let episode = episode.to_lowercase();
  let target = target.to_lowercase();

  let (
    target_frame,
    target_format
  ) = match target.split_once(".") {
    Some(r) => r,
    None => return StatusCode::BAD_REQUEST.into_response(),
  };
  
  let animated_frame = target_frame.split_once("-");

  let target_format = match target_format {
    "png" => ImageFormat::Png,
    "gif" => ImageFormat::Gif,
    "webp" => ImageFormat::WebP,
    "jpg" | "jpeg" => ImageFormat::Jpeg,
    _ => return StatusCode::BAD_REQUEST.into_response()
  };

  if animated_frame.is_some() && target_format != ImageFormat::Gif {
    return StatusCode::BAD_REQUEST.into_response();
  }

  let env_config = match ENV_CONFIG.get() {
    Some(env) => env,
    None => return StatusCode::INTERNAL_SERVER_ERROR.into_response()
  };

  let season_name = match season.as_str() {
    "1" | "mygo" => "",
    "2" | "ave" | "ave-mujica" => "ave-",
    _ => ""
  };

  if let Some(animated_frame) = animated_frame {
    return handle_animated_image(
      env_config,
      &season_name,
      &episode,
      animated_frame,
      scheduler
    ).await;
  } else {
    return handle_static_image(
      env_config,
      &season_name,
      &episode,
      target_frame,
      target_format
    ).await;
  }
}

async fn handle_animated_image(
  env_config: &EnvConfig,
  season_name: &str,
  episode: &str,
  animated_frame: (&str, &str),
  scheduler: Arc<Scheduler>
) -> Response<Body> {
  let u32_frame: Option<(u32, u32)> = animated_frame.0.parse().ok()
    .zip(animated_frame.1.parse().ok());

  let (start_frame, end_frame) = match u32_frame {
    Some(r) => r,
    None => return StatusCode::BAD_REQUEST.into_response(),
  };

  if start_frame >= end_frame || start_frame <= 0 {
    return StatusCode::BAD_REQUEST.into_response();
  }

  let frames = end_frame - start_frame;
  
  if let Some(animate_frame_limit) = &env_config.animate_frame_limit {
    if frames >= *animate_frame_limit {
      return (
        StatusCode::PAYLOAD_TOO_LARGE,
        "大於 150 秒（3600 幀）的片段無法透過此 API 請求，請向開發者提交片段投稿"
      ).into_response();
    }
  }

  convert_animated_image(
    env_config,
    start_frame,
    frames,
    &season_name,
    &episode,
    scheduler
  ).await
}

async fn handle_static_image(
  env_config: &EnvConfig,
  season_name: &str,
  episode: &str,
  target_frame: &str,
  target_format: ImageFormat
) -> Response<Body> {
  let source_file_path = format!(
    "{}/{}{}_{}.webp",
    env_config.image_source_path,
    season_name,
    episode,
    target_frame
  );

  if let Ok(exists) = fs::try_exists(&source_file_path).await {
    if !exists {
      return StatusCode::NOT_FOUND.into_response();
    }
  } else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let reader = match fs::read(&source_file_path).await {
    Ok(img) => Cursor::new(img),
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  convert_static_image(reader, target_format).await
}
