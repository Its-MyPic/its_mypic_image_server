use std::io::{BufWriter, Cursor};

use axum::{body::Body, http::{Response, StatusCode}, response::IntoResponse};
use image::{imageops::FilterType, ImageFormat, ImageReader};
use tokio::fs;
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
  episode: &str,
  target_format: ImageFormat
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

  match target_format {
    ImageFormat::Png => return encode_apng(
      env_config,
      start_frame,
      end_frame,
      season_name,
      episode
    ).await,
    ImageFormat::Gif => return encode_gif(
      env_config,
      start_frame,
      end_frame,
      season_name,
      episode
    ).await,
    ImageFormat::WebP => return encode_anim_webp(
      env_config,
      start_frame,
      end_frame,
      season_name,
      episode
    ).await,
    _ => return StatusCode::BAD_REQUEST.into_response(),
  }
}

async fn encode_gif(
  env_config: &EnvConfig,
  start_frame: u32,
  end_frame: u32,
  season_name: &str,
  episode: &str
) -> Response<Body> {
  if end_frame - start_frame >= 120 {
    return (
      StatusCode::PAYLOAD_TOO_LARGE,
      "大於 5 秒（120 幀）的片段無法透過此 API 請求 GIF 格式，請向開發者提交片段投稿"
    ).into_response();
  }

  let mut encoder = match gif::Encoder::new(
    BufWriter::new(Vec::new()),
    854,
    480,
    &[]
  ) {
    Ok(enc) => enc,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  match encoder.write_extension(
    gif::ExtensionData::new_control_ext(
      4,
      gif::DisposalMethod::Previous,
      false,
      None
    )
  ) {
    Ok(_) => {},
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }
  
  match encoder.write_extension(
    gif::ExtensionData::Repetitions(
      gif::Repeat::Infinite
    )
  ) {
    Ok(_) => {},
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }

  for frame_idx in start_frame ..= end_frame {
    let img = match image::open(
      format!(
        "{}/{}{}_{}.webp",
        env_config.image_source_path,
        season_name,
        episode,
        frame_idx
      )
    ) {
      Ok(img) => img,
      Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mut buf = match img.resize_exact(
      854,
      480,
      FilterType::Triangle
    ).as_rgb8() {
      Some(buf) => buf.to_vec(),
      None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match encoder.write_frame(
      &gif::Frame::from_rgb_speed(
        854,
        480,
        &mut buf,
        25
      )
    ) {
      Ok(_) => {},
      Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
  }

  let writer = match encoder.into_inner() {
    Ok(w) => w,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  let output_buf = match writer.into_inner() {
    Ok(buf) => buf,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  return (
    StatusCode::OK,
    Body::from_stream(
      ReaderStream::new(
        Cursor::new(
          output_buf
        )
      )
    )
  ).into_response();
}

async fn encode_apng(
  env_config: &EnvConfig,
  start_frame: u32,
  end_frame: u32,
  season_name: &str,
  episode: &str
) -> Response<Body> {
  if end_frame - start_frame >= 240 {
    return (
      StatusCode::PAYLOAD_TOO_LARGE,
      "大於 10 秒（240 幀）的片段無法透過此 API 請求 APNG 格式，請向開發者提交片段投稿"
    ).into_response();
  }

  let mut output_writer = BufWriter::new(Vec::new());

  let mut encoder = png::Encoder::new(
    &mut output_writer,
    854,
    480
  );

  encoder.set_color(
    png::ColorType::Rgb
  );
  match encoder.set_animated(
    end_frame - start_frame + 1,
    0
  ) {
    Ok(_) => {},
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }
  match encoder.set_frame_delay(
    1,
    24
  ) {
    Ok(_) => {},
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }

  let mut writer = match encoder.write_header() {
    Ok(w) => w,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  for frame_idx in start_frame ..= end_frame {
    let img = match image::open(
      format!(
        "{}/{}{}_{}.webp",
        env_config.image_source_path,
        season_name,
        episode,
        frame_idx
      )
    ) {
      Ok(img) => img,
      Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let buf = match img.resize_exact(
      854,
      480,
      FilterType::Triangle
    ).as_rgb8() {
      Some(buf) => buf.to_vec(),
      None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match writer.write_image_data(&buf) {
      Ok(_) => {},
      Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
  }

  match writer.finish() {
    Ok(_) => {},
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  }

  let output_buf = match output_writer.into_inner() {
    Ok(buf) => buf,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  return (
    StatusCode::OK,
    Body::from_stream(
      ReaderStream::new(
        Cursor::new(
          output_buf
        )
      )
    )
  ).into_response();
}

async fn encode_anim_webp(
  env_config: &EnvConfig,
  start_frame: u32,
  end_frame: u32,
  season_name: &str,
  episode: &str
) -> Response<Body> {
  if end_frame - start_frame >= 120 {
    return (
      StatusCode::PAYLOAD_TOO_LARGE,
      "大於 5 秒（120 幀）的片段無法透過此 API 請求 WebP 格式，請向開發者提交片段投稿"
    ).into_response();
  }

  let mut encoder = match webp_animation::Encoder::new_with_options(
    (854, 480),
    webp_animation::EncoderOptions {
      color_mode: webp_animation::ColorMode::Rgb,
      allow_mixed: true,
      encoding_config: Some(
        webp_animation::EncodingConfig {
          quality: 75.0,
          method: 2,
          ..Default::default()
        }
      ),
      ..Default::default()
    }
  ) {
    Ok(enc) => enc,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  let mut anim_timestemp = 0;

  for frame_idx in start_frame ..= end_frame {
    let img = match image::open(
      format!(
        "{}/{}{}_{}.webp",
        env_config.image_source_path,
        season_name,
        episode,
        frame_idx
      )
    ) {
      Ok(img) => img,
      Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let buf = match img.resize_exact(
      854,
      480,
      FilterType::Triangle
    ).as_rgb8() {
      Some(buf) => buf.to_vec(),
      None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match encoder.add_frame(
      &buf,
      anim_timestemp
    ) {
      Ok(_) => {},
      Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }

    anim_timestemp += 42;
  }

  let output_buf = match encoder.finalize(anim_timestemp) {
    Ok(buf) => buf,
    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
  };

  return (
    StatusCode::OK,
    Body::from_stream(
      ReaderStream::new(
        Cursor::new(
          output_buf
        )
      )
    )
  ).into_response();
}
