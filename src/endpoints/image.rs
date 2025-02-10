use std::io::Cursor;

use axum::{
  body::Body, extract::Path, http::{Response, StatusCode}, response::IntoResponse
};
use image::ImageFormat;
use regex::Regex;
use tokio::{fs, sync::OnceCell};

use crate::utils::{convert::{convert_animated_image, convert_static_image}, env::{EnvConfig, ENV_CONFIG}};


static URL_FILE_REGEX: OnceCell<Regex> = OnceCell::const_new();


pub(crate) async fn handler(
  Path((season, episode, target)): Path<(String, String, String)>,
) -> impl IntoResponse {
  let season = season.to_lowercase();
  let episode = episode.to_lowercase();
  let target = target.to_lowercase();

  let regex = match URL_FILE_REGEX.get_or_try_init(
    || async {
      Regex::new(r"(?P<target_frame>[0-9]*)\.(?P<target_format>jpg|jpeg|png|webp)|(?P<target_anim_frame>[0-9]*-[0-9]*)\.(?P<target_anim_format>png|webp|apng|gif)")
    }
  ).await {
    Ok(regex) => regex,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  let (
    target_frame,
    target_format,
    is_animated
  ) = match regex.captures(&target) {
    Some(result) => {
      let result = result
        .name("target_frame")
        .zip(result.name("target_format"))
        .zip(Some(false))
        .or(
          result.name("target_anim_frame")
          .zip(result.name("target_anim_format"))
          .zip(Some(true))
        );

      if let Some(result) = result {
        (
          result.0.0.as_str(),
          result.0.1.as_str(),
          result.1
        )
      } else {
        return StatusCode::BAD_REQUEST.into_response();
      }
    },
    None => return StatusCode::BAD_REQUEST.into_response()
  };

  let env_config = match ENV_CONFIG.get() {
    Some(env) => env,
    None => return StatusCode::INTERNAL_SERVER_ERROR.into_response()
  };

  let season_name = match season.as_str() {
    "mygo" => "",
    "ave" | "ave-mujica" => "ave-",
    _ => ""
  };

  if is_animated {
    handle_animated_image(
      env_config,
      &season_name,
      &episode,
      target_frame,
      target_format
    ).await
  } else {
    handle_static_image(
      env_config,
      &season_name,
      &episode,
      target_frame,
      target_format
    ).await
  }
}

async fn handle_animated_image(
  env_config: &EnvConfig,
  season_name: &str,
  episode: &str,
  target_frame: &str,
  target_format: &str
) -> Response<Body> {
  let (start_frame, end_frame) = match target_frame.split_once("-") {
    Some(r) => {
      match r.0.parse::<u32>().ok().zip(r.1.parse::<u32>().ok()) {
        Some(r) => r,
        None => return StatusCode::BAD_REQUEST.into_response(),
      }
    },
    None => return StatusCode::BAD_REQUEST.into_response()
  };

  if start_frame >= end_frame {
    return StatusCode::BAD_REQUEST.into_response();
  }

  let target_format = match target_format {
    "png" | "apng" => ImageFormat::Png,
    "gif" => ImageFormat::Gif,
    "webp" => ImageFormat::WebP,
    _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()
  };

  convert_animated_image(
    env_config,
    start_frame,
    end_frame,
    &season_name,
    &episode,
    target_format
  ).await
}

async fn handle_static_image(
  env_config: &EnvConfig,
  season_name: &str,
  episode: &str,
  target_frame: &str,
  target_format: &str
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

  let target_format = match target_format {
    "jpg" | "jpeg" => ImageFormat::Jpeg,
    "png" => ImageFormat::Png,
    "webp" => ImageFormat::WebP,
    _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()
  };

  convert_static_image(reader, target_format).await
}
