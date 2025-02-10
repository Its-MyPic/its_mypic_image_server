use chrono::{DateTime, Local};
use tracing_subscriber::fmt::{format, time};

pub(crate) struct CustomLogTimer;

impl time::FormatTime for CustomLogTimer {
  fn format_time(&self, w: &mut format::Writer<'_>) -> std::fmt::Result {
    let current_local: DateTime<Local> = Local::now();
    write!(w, "{}", current_local.format("%F %T%.3f"))
  }
}
