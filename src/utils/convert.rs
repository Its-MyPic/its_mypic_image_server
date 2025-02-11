use std::{io::Cursor, process::Stdio};

use axum::{body::Body, http::{Response, StatusCode}, response::IntoResponse};
use image::{ImageFormat, ImageReader};
use tokio::{fs, process::Command};
use tokio_util::io::ReaderStream;

use super::env::EnvConfig;


static SOURCE_FORMAT: ImageFormat = ImageFormat::WebP;


pub(crate) async fn convert_static_image(
  reader: Cursor<Vec<u8>>,
  target_format: ImageFormat
) -> Response<Body> {
  if target_format == SOURCE_FORMAT {
    return (
      StatusCode::OK,
      Body::from_stream(
        ReaderStream::new(reader)
      )
    ).into_response();
  }

  let mut buf = Cursor::new(Vec::new());
  let mut img_reader = ImageReader::new(reader);
  img_reader.set_format(SOURCE_FORMAT);

  let decoded_img = match img_reader.decode() {
    Ok(img) => img,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  match decoded_img.write_to(&mut buf, target_format) {
    Ok(_) => {
      buf.set_position(0);

      return (
        StatusCode::OK,
        Body::from_stream(
          ReaderStream::new(buf)
        )
      ).into_response();
    },
    Err(_) => {
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    },
  };
}

pub(crate) async fn convert_animated_image(
  env_config: &EnvConfig,
  start_frame: u32,
  end_frame: u32,
  season_name: &str,
  episode: &str
) -> Response<Body> {
  let start_fram_file_path = format!(
    "{}/{}{}_{}.webp",
    env_config.image_source_path,
    season_name,
    episode,
    start_frame
  );

  let end_fram_file_path = format!(
    "{}/{}{}_{}.webp",
    env_config.image_source_path,
    season_name,
    episode,
    start_frame
  );

  let is_segment_exists = fs::try_exists(&start_fram_file_path)
  .await
  .ok()
  .zip(
    fs::try_exists(&end_fram_file_path)
    .await
    .ok()
  );

  if let Some(exists) = is_segment_exists {
    if !exists.0 || !exists.1 {
      return StatusCode::NOT_FOUND.into_response();
    }
  } else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  encode_gif(
    env_config,
    start_frame,
    end_frame,
    season_name,
    episode
  ).await
}

async fn encode_gif(
  env_config: &EnvConfig,
  start_frame: u32,
  frames: u32,
  season_name: &str,
  episode: &str
) -> Response<Body> {
  let file_pattern = format!(
    "{}/{}{}_%d.webp",
    env_config.image_source_path,
    season_name,
    episode
  );

  let output = Command::new("ffmpeg")
    .args(
      [
        "-i", &file_pattern,
        "-start_number", &start_frame.to_string(),
        "-frames:v", &frames.to_string(),
        "-f", "gif",
        "-framerate", "24",
        "pipe:1"
      ]
    )
    .stdout(Stdio::piped())
    .output()
    .await
    .unwrap();

  return (
    StatusCode::OK,
    Body::from_stream(
      ReaderStream::new(
        Cursor::new(
          output.stdout
        )
      )
    )
  ).into_response();
}
