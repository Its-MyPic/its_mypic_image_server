use std::{io::Cursor, sync::Arc};

use axum::{
  body::Body,
  http::{Response, StatusCode},
  response::IntoResponse
};
use crossbeam::channel::bounded;
use image::{ImageFormat, ImageReader};
use tokio::fs;
use tokio_util::io::ReaderStream;

use crate::Scheduler;

use super::{
  env::EnvConfig,
  task::{Task, TaskData}
};


static SOURCE_FORMAT: ImageFormat = ImageFormat::WebP;


pub(crate) async fn convert_static_image(
  reader: Cursor<Vec<u8>>,
  format: ImageFormat
) -> Response<Body> {
  if format == SOURCE_FORMAT {
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
    Err(_) => return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to decode source image."
    ).into_response(),
  };

  match decoded_img.write_to(&mut buf, format) {
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
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to write encoded image to response buffer."
      ).into_response();
    },
  };
}

pub(crate) async fn convert_animated_image(
  env_config: &EnvConfig,
  start_frame: u32,
  frames: u32,
  season: &str,
  episode: &str,
  scheduler: Arc<Scheduler>
) -> Response<Body> {
  let start_fram_file_path = format!(
    "{}/{}{}_{}.webp",
    env_config.image_source_path,
    season,
    episode,
    start_frame
  );

  let end_fram_file_path = format!(
    "{}/{}{}_{}.webp",
    env_config.image_source_path,
    season,
    episode,
    start_frame + frames
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
      return (
        StatusCode::NOT_FOUND,
        "Start or end image file not exists."
      ).into_response();
    }
  } else {
    return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to read start or end image file."
    ).into_response();
  }

  let file_pattern = format!(
    "{}/{}{}_%d.webp",
    env_config.image_source_path,
    season,
    episode
  );

  let (send, recv) = bounded(1);

  let task = Task::new(
    TaskData::new(
      start_frame,
      frames,
      file_pattern,
      send
    )
  );

  scheduler.add_task(task.clone());

  if let Ok(data) = recv.recv() {
    return (
      StatusCode::OK,
      Body::from_stream(
        ReaderStream::new(
          Cursor::new(
            data
          )
        )
      )
    ).into_response();
  } else {
    return (
      StatusCode::INTERNAL_SERVER_ERROR,
      "Failed to receive data from channel."
    ).into_response();
  }
}
