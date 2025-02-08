use std::io::Cursor;

use axum::{body::Body, http::{Response, StatusCode}, response::IntoResponse};
use image::{ImageFormat, ImageReader};
use tokio_util::io::ReaderStream;


static SOURCE_FORMAT: ImageFormat = ImageFormat::WebP;


pub(crate) fn convert_image(reader: Cursor<Vec<u8>>, target_format: ImageFormat) -> Response<Body> {
  if target_format == SOURCE_FORMAT {
    return (StatusCode::OK, Body::from_stream(ReaderStream::new(reader))).into_response();
  }

  let mut buf = Cursor::new(Vec::new());
  let mut img_reader = ImageReader::new(reader);
  img_reader.set_format(SOURCE_FORMAT);

  match img_reader.decode().unwrap().write_to(&mut buf, target_format) {
    Ok(_) => {
      buf.set_position(0);
      return (StatusCode::OK, Body::from_stream(ReaderStream::new(buf))).into_response();
    },
    Err(_) => {
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    },
  };
}
