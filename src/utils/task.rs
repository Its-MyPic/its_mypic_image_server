use std::{
  process::{Command, Stdio},
  sync::Arc,
  time
};

use parking_lot::{Mutex, RwLock, Condvar};
use tracing::info;


#[derive(Debug)]
pub(crate) struct Semaphore {
  flag_count: Mutex<u32>,
  max_flag: u32,
  notifier: Condvar
}

impl Semaphore {
  pub(crate) fn new(init_flag: u32, max_flag: u32) -> Self {
    Self {
      flag_count: Mutex::new(init_flag),
      max_flag,
      notifier: Condvar::new()
    }
  }

  pub(crate) fn wait(self: &Self) {
    let mut flag_count = self.flag_count.lock();

    if *flag_count <= 0 {
      self.notifier.wait(&mut flag_count);
    }

    *flag_count -= 1;

    assert!(
      *flag_count <= self.max_flag,
      "wait() paincked, flag_count = {}, max_flag = {}",
      flag_count,
      self.max_flag
    );
  }

  pub(crate) fn release(self: &Self) {
    let mut flag_count = self.flag_count.lock();

    *flag_count += 1;
    self.notifier.notify_one();

    assert!(
      *flag_count <= self.max_flag,
      "release() paincked, flag_count = {}, max_flag = {}",
      flag_count,
      self.max_flag
    );
  }
}

#[derive(Debug)]
pub(crate) struct MonoSemaphore {
  flag: Mutex<bool>,
  notifier: Condvar
}

impl MonoSemaphore {
  pub(crate) fn new(init_flag: bool) -> Self {
    Self {
      flag: Mutex::new(init_flag),
      notifier: Condvar::new()
    }
  }

  pub(crate) fn wait(self: &Self) {
    let mut flag = self.flag.lock();

    if !*flag {
      self.notifier.wait(&mut flag);
    }
  }

  pub(crate) fn take(self: &Self) {
    let mut flag = self.flag.lock();

    *flag = false;
    self.notifier.notify_one();
  }

  pub(crate) fn release(self: &Self) {
    let mut flag = self.flag.lock();

    *flag = true;
    self.notifier.notify_one();
  }
}

pub(crate) struct Scheduler {
  loop_sem: MonoSemaphore,
  task_sem: Semaphore,
  tasks: RwLock<Vec<Arc<Task>>>
}

impl Scheduler {
  pub(crate) fn new(max_process: u32) -> Arc<Self> {
    Arc::new(
      Self {
        loop_sem: MonoSemaphore::new(false),
        task_sem: Semaphore::new(max_process, max_process),
        tasks: RwLock::new(Vec::new()),
      }
    )
  }

  pub(crate) fn start(self: &Arc<Self>) {
    let scheduler = self.clone();
    info!(" [ Sch ]  Spawn Scheduler Polling Process");
    tokio::task::spawn_blocking(
      move || loop {
        scheduler.loop_sem.wait();

        if scheduler.tasks.read().len() > 0 {
          scheduler.task_sem.wait();

          let task = scheduler.tasks
            .write()
            .pop()
            .unwrap_or_else(|| panic!("Failed to get task"));

          let inner_scheduler = scheduler.clone();

          info!(" [ Sch ]  Spawn FFMPEG Task");
          tokio::task::spawn_blocking(
            move || {
              task.execute();

              inner_scheduler.task_sem.release();
            }
          );
        } else {
          scheduler.loop_sem.take();
        }
      }
    );
  }

  pub(crate) fn add_task(self: &Arc<Self>, task: Arc<Task>) {
    self.tasks.write().push(task);

    self.loop_sem.release();
  }
}

#[derive(Debug)]
pub(crate) struct Task {
  pub(crate) sem: MonoSemaphore,
  pub(crate) data: TaskData,
  _ts: u128
}

impl Task {
  pub(crate) fn new(data: TaskData) -> Arc<Self> {
    Arc::new(
      Self {
        sem: MonoSemaphore::new(false),
        data,
        _ts: time::SystemTime::now().duration_since(
          time::UNIX_EPOCH
        ).unwrap().as_millis()
      }
    )
  }

  fn execute(self: &Arc<Self>) {
    *self.data.output.write() = Command::new("ffmpeg")
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

    self.sem.release();
  }
}

#[derive(Debug)]
pub(crate) struct TaskData {
  start_frame: u32,
  frames: u32,
  file_pattern: String,
  pub(crate) output: RwLock<Vec<u8>>
}

impl TaskData {
  pub(crate) fn new(
    start_frame: u32,
    frames: u32,
    file_pattern: String,
    output: RwLock<Vec<u8>>
  ) -> Self {
    Self {
      start_frame,
      frames,
      file_pattern,
      output
    }
  }
}
