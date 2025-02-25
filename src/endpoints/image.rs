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


static EPISODE_SPLIT_FRAME: [u32; 3] = [0, 34288, 68334];


pub(crate) async fn handler(
  Path((season, episode, target)): Path<(u8, String, String)>,
  State(scheduler): State<Arc<Scheduler>>,
) -> impl IntoResponse {
  let season = match season {
    1 => "",
    2 => "ave-",
    _ => ""
  };

  let mut episode = episode.to_lowercase();
  let target = target.to_lowercase();

  let env_config = match ENV_CONFIG.get() {
    Some(env) => env,
    None => return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to load server env."
    ).into_response()
  };

  let (
    target_frame,
    target_format
  ) = match target.split_once(".") {
    Some(r) => r,
    None => return (
      StatusCode::BAD_REQUEST,
      "Failed to parse target file."
    ).into_response(),
  };

  let target_format = match target_format {
    "png" => ImageFormat::Png,
    "gif" => ImageFormat::Gif,
    "webp" => ImageFormat::WebP,
    "jpg" | "jpeg" => ImageFormat::Jpeg,
    _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()
  };
  
  let mut animated_frame: Option<(u32, u32)> = target_frame
    .split_once("-")
    .and_then(
      |r| r.0.parse().ok().zip(r.1.parse().ok())
    );

  let mut frame = target_frame.parse().ok();

  if animated_frame.is_some() && target_format != ImageFormat::Gif {
    return (
      StatusCode::BAD_REQUEST,
      "Cannot request static file with frame range."
    ).into_response();
  }

  if season.is_empty() {
    let offset = match episode.as_str() {
      "1" => Some(EPISODE_SPLIT_FRAME[0]),
      "2" => Some(EPISODE_SPLIT_FRAME[1]),
      "3" => Some(EPISODE_SPLIT_FRAME[2]),
      _ => None
    };

    if let Some(offset) = offset {
      match (frame, animated_frame) {
        (Some(f), None) => {
          frame = Some(f + offset)
        },
        (None, Some(a_f)) => {
          animated_frame = Some((a_f.0 + offset, a_f.1 + offset))
        },
        _ => return (
          StatusCode::BAD_REQUEST,
          "Failed to request file with target frame."
        ).into_response()
      }
      episode = String::from("1-3");
    }
  }
  
  match (frame, animated_frame) {
    (Some(frame), None) => return handle_static_image(
      env_config,
      season,
      &episode,
      frame,
      target_format
    ).await,
    (None, Some(animated_frame)) => 
    return handle_animated_image(
      env_config,
      season,
      &episode,
      animated_frame,
      scheduler
    ).await,
    _ => return (
      StatusCode::BAD_REQUEST,
      "Failed to request file with target frame."
    ).into_response()
  }
}

async fn handle_animated_image(
  env_config: &EnvConfig,
  season: &str,
  episode: &str,
  animated_frame: (u32, u32),
  scheduler: Arc<Scheduler>
) -> Response<Body> {
  let (start_frame, end_frame) = animated_frame;

  if start_frame >= end_frame || start_frame <= 0 {
    return (
      StatusCode::BAD_REQUEST,
      "Invalid frame range (left >= right || left <= 0)."
    ).into_response();
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
    season,
    &episode,
    scheduler
  ).await
}

async fn handle_static_image(
  env_config: &EnvConfig,
  season: &str,
  episode: &str,
  frame: u32,
  format: ImageFormat
) -> Response<Body> {
  let source_file_path = format!(
    "{}/{}{}_{}.webp",
    env_config.image_source_path,
    season,
    episode,
    frame
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

  convert_static_image(reader, format).await
}
