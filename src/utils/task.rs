use std::{process::{Command, Stdio}, sync::Arc, time};

use parking_lot::{Mutex, RwLock, Condvar};


pub(crate) struct Scheduler {
  sem: Arc<(Mutex<u32>, Condvar)>,
  loop_sem: Arc<(Mutex<bool>, Condvar)>,
  tasks: Arc<RwLock<Vec<Arc<RwLock<Task>>>>>
}

impl Scheduler {
  pub(crate) fn new(max_process: u32) -> Self {
    Scheduler {
      sem: Arc::new((Mutex::new(max_process), Condvar::new())),
      loop_sem: Arc::new((Mutex::new(false), Condvar::new())),
      tasks: Arc::new(RwLock::new(Vec::new())),
    }
  }

  pub(crate) fn start(self: &mut Self) {
    let sem = self.sem.clone();
    let loop_sem = self.loop_sem.clone();
    let tasks = self.tasks.clone();
    tokio::task::spawn_blocking(
      move || loop {
        let (
          loop_cmtx,
          loop_cvar
        ) = &*loop_sem;

        let mut waked = loop_cmtx.lock();
        if !*waked {
          loop_cvar.wait(&mut waked);
        }

        let pop_task = tasks.write().pop();

        if let Some(task) = pop_task {
          let inner_sem = sem.clone();
          let (
            cmtx,
            cvar
          ) = &*sem;

          let mut free_worker = cmtx.lock();
          if !(*free_worker > 0) {
            cvar.wait(&mut free_worker);
          }

          *free_worker -= 1;

          tokio::task::spawn_blocking(
            move || {
              let (
                cmtx,
                cvar
              ) = &*inner_sem;

              task.write().execute();

              *cmtx.lock() += 1;
              cvar.notify_one();
            }
          );
        } else {
          *waked = false;
        }
      }
    );
  }

  pub(crate) fn add_task(self: &mut Self, task: Arc<RwLock<Task>>) {
    let (
      loop_cmtx,
      loop_cvar
    ) = &*self.loop_sem.clone();

    self.tasks.write().push(task);

    *loop_cmtx.lock() = true;
    loop_cvar.notify_one();
  }
}

pub(crate) struct Task {
  pub(crate) sem: Arc<(Mutex<bool>, Condvar)>,
  pub(crate) data: TaskData,
  _ts: u128
}

impl Task {
  pub(crate) fn new(data: TaskData) -> Self {
    Self {
      sem: Arc::new((Mutex::new(false), Condvar::new())),
      data,
      _ts: time::SystemTime::now().duration_since(
        time::UNIX_EPOCH
      ).unwrap().as_millis()
    }
  }

  fn execute(self: &mut Self) {
    let (ref cmtx, ref cvar) = &*self.sem;

    self.data.output = Command::new("ffmpeg")
      .args(
        [
          "-start_number", &self.data.start_frame.to_string(),
          "-i", &self.data.file_pattern,
          "-frames:v", &self.data.frames.to_string(),
          "-f", "gif",
          "-framerate", "24",
          "pipe:1"
        ]
      )
      .stdout(Stdio::piped())
      .output()
      .unwrap_or_else(|_| panic!("FFMPEG Error"))
      .stdout;

    *cmtx.lock() = true;
    cvar.notify_one();
  }
}

#[derive(Debug)]
pub(crate) struct TaskData {
  start_frame: u32,
  frames: u32,
  file_pattern: String,
  pub(crate) output: Vec<u8>
}

impl TaskData {
  pub(crate) fn new(
    start_frame: u32,
    frames: u32,
    file_pattern: String,
    output: Vec<u8>
  ) -> Self {
    Self {
      start_frame,
      frames,
      file_pattern,
      output
    }
  }
}
