use std::{
  process::{Command, Stdio},
  sync::Arc,
  time
};

use crossbeam::channel::Sender;
use parking_lot::{Mutex, RwLock, Condvar};
use priority_queue::DoublePriorityQueue;
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
  tasks: RwLock<DoublePriorityQueue<Arc<Task>, u32>>
}

impl Scheduler {
  pub(crate) fn new(max_process: u32) -> Arc<Self> {
    Arc::new(
      Self {
        loop_sem: MonoSemaphore::new(false),
        task_sem: Semaphore::new(max_process, max_process),
        tasks: RwLock::new(DoublePriorityQueue::new()),
      }
    )
  }

  pub(crate) fn start(self: &Arc<Self>) {
    let scheduler = self.clone();
    info!(" [ Sch ]  Spawn Scheduler Polling Process");
    tokio::task::spawn_blocking(
      move || loop {
        scheduler.loop_sem.wait();

        if !scheduler.tasks.read().is_empty() {
          scheduler.task_sem.wait();

          let (task, _) = scheduler.tasks
            .write()
            .pop_min()
            .unwrap_or_else(
              || {
                info!(" [ Sch ]  Scheduler Panicked! Failed to Get Task");
                panic!();
              }
            );

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
    let priority = task.data.frames;

    self.tasks.write().push(task, priority);

    self.loop_sem.release();
  }
}

#[derive(Debug)]
pub(crate) struct Task {
  pub(crate) data: TaskData,
  _ts: u128
}

impl std::hash::Hash for Task {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.data.hash(state);
    self._ts.hash(state);
  }
}

impl std::cmp::PartialEq for Task {
  fn eq(&self, other: &Self) -> bool {
    self.data == other.data &&
    self._ts == other._ts
  }
}

impl std::cmp::Eq for Task {}

impl Task {
  pub(crate) fn new(data: TaskData) -> Arc<Self> {
    Arc::new(
      Self {
        data,
        _ts: time::SystemTime::now().duration_since(
          time::UNIX_EPOCH
        ).unwrap().as_millis()
      }
    )
  }

  fn execute(self: &Arc<Self>) {
    self.data.sender.send(
      Command::new("ffmpeg")
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
      .unwrap_or_else(
        |_| {
          info!(" [ Tsk ]  Task Panicked! FFMPEG Error");
          panic!();
        }
      )
      .stdout
    ).unwrap_or_else(
      |_| {
        info!(" [ Tsk ]  Task Panicked! Channel Error");
        panic!();
      }
    );
  }
}

#[derive(Debug)]
pub(crate) struct TaskData {
  start_frame: u32,
  frames: u32,
  file_pattern: String,
  pub(crate) sender: Sender<Vec<u8>>
}

impl std::hash::Hash for TaskData {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.start_frame.hash(state);
    self.frames.hash(state);
    self.file_pattern.hash(state);
  }
}

impl std::cmp::PartialEq for TaskData {
  fn eq(&self, other: &Self) -> bool {
    self.start_frame == other.start_frame &&
    self.frames == other.frames &&
    self.file_pattern == other.file_pattern
  }
}

impl std::cmp::Eq for TaskData {}

impl TaskData {
  pub(crate) fn new(
    start_frame: u32,
    frames: u32,
    file_pattern: String,
    sender: Sender<Vec<u8>>
  ) -> Self {
    Self {
      start_frame,
      frames,
      file_pattern,
      sender
    }
  }
}
