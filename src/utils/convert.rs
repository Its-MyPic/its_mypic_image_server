use std::{io::Cursor, sync::Arc};

use axum::{
  body::Body,
  http::{Response, StatusCode},
  response::IntoResponse
};
use image::{ImageFormat, ImageReader};
use parking_lot::RwLock;
use tokio::fs;
use tokio_util::io::ReaderStream;

use crate::Scheduler;

use super::{env::EnvConfig, task::{Task, TaskData}};


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
  frames: u32,
  season_name: &str,
  episode: &str,
  state: Arc<RwLock<Scheduler>>
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
      return StatusCode::NOT_FOUND.into_response();
    }
  } else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  let file_pattern = format!(
    "{}/{}{}_%d.webp",
    env_config.image_source_path,
    season_name,
    episode
  );

  let output = Vec::new();

  let task = Arc::new(
    RwLock::new(
      Task::new(
        TaskData::new(
          start_frame,
          frames,
          file_pattern,
          output
        )
      )
    )
  );

  let task_send = task.clone();
  state.write().add_task(task_send);
  
  let (
    cmtx,
    cvar
  ) = &*task.read().sem.clone();

  let mut done = cmtx.lock();
  if !*done {
    cvar.wait(&mut done);
  }

  if let Ok(task) = Arc::try_unwrap(task) {
    return (
      StatusCode::OK,
      Body::from_stream(
        ReaderStream::new(
          Cursor::new(
            task.into_inner().data.output
          )
        )
      )
    ).into_response();
  } else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
}
