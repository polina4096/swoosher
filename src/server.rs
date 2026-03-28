use std::{
  io::{BufRead, Write},
  os::unix::net::UnixListener,
  time::Duration,
};

use camino::Utf8Path;
use color_eyre::eyre::{self, Context as _};
use tap::Tap;

use crate::spaces::{self, Direction};

pub struct Server {
  listener: UnixListener,
  timeout: Duration,
}

impl Server {
  pub fn bind(socket_path: &Utf8Path, timeout_secs: u64) -> eyre::Result<Self> {
    if let Some(parent) = socket_path.parent() {
      fs_err::create_dir_all(parent)?;
    }

    if socket_path.exists() {
      fs_err::remove_file(socket_path)?;
    }

    let timeout = std::time::Duration::from_secs(timeout_secs);

    return Ok(Self {
      listener: UnixListener::bind(socket_path.as_std_path())
        .context("failed to bind Unix socket")?
        .tap(|_| log::info!("Listening on {socket_path}")),
      timeout,
    });
  }

  pub fn run(&self) -> eyre::Result<()> {
    loop {
      let Ok(stream) = { self.listener.accept() }
        .inspect_err(|e| log::warn!("Accept error: {e}"))
        .map(|(stream, _)| stream)
      else {
        continue;
      };

      if self.timeout.as_secs() > 0 {
        stream.set_read_timeout(Some(self.timeout)).ok();
      }

      std::thread::spawn(move || {
        if let Err(e) = handle_client(stream) {
          log::debug!("Client disconnected: {e}");
        }
      });
    }
  }
}

fn handle_client(stream: std::os::unix::net::UnixStream) -> eyre::Result<()> {
  let mut writer = stream.try_clone().ok();
  let reader = std::io::BufReader::new(stream);

  for line in reader.lines() {
    let line = line.context("failed to read line")?;
    if line.is_empty() {
      continue;
    }

    handle_command(&line, writer.as_mut());
  }

  return Ok(());
}

fn handle_command(cmd: &str, writer: Option<&mut std::os::unix::net::UnixStream>) {
  let cmd = cmd.trim();

  log::debug!("Command: {cmd}");

  match cmd {
    "left" => switch_direction(Direction::Left),
    "right" => switch_direction(Direction::Right),
    "info" => {
      if let Some(w) = writer {
        respond_space_info(w);
      }
    }
    _ => {
      match cmd.split_once(' ') {
        Some(("index", n)) => {
          if let Ok(target) = n.parse::<u32>() {
            switch_to_index(target.saturating_sub(1));
          }
        }

        _ => log::warn!("Unknown command: {cmd}"),
      }
    }
  }
}

fn respond_space_info(writer: &mut impl Write) {
  let _ = match spaces::space_info() {
    Some(info) => writeln!(writer, "{} {}", info.index + 1, info.count),
    None => writeln!(writer, "error"),
  };
}

fn switch_direction(direction: Direction) {
  if let Some(info) = spaces::space_info() {
    match direction {
      Direction::Left if info.index == 0 => return,
      Direction::Right if info.index + 1 >= info.count => return,
      _ => {}
    }
  }

  spaces::post_switch_gesture(direction);
}

fn switch_to_index(target: u32) {
  let Some(info) = spaces::space_info()
  else {
    return;
  };

  let target = target.min(info.count.saturating_sub(1));
  if info.index == target {
    return;
  }

  let (direction, steps) = match info.index < target {
    true => (Direction::Right, target - info.index),
    false => (Direction::Left, info.index - target),
  };

  for _ in 0 .. steps {
    spaces::post_switch_gesture(direction);
  }
}

/// Direct call: switch left.
pub fn switch_left() {
  switch_direction(Direction::Left);
}

/// Direct call: switch right.
pub fn switch_right() {
  switch_direction(Direction::Right);
}

/// Direct call: switch to 1-based index.
pub fn switch_to(index_1based: u32) {
  switch_to_index(index_1based.saturating_sub(1));
}

/// Direct call: get space info. Returns (current_1based, count).
pub fn get_space_info() -> Option<(u32, u32)> {
  let info = spaces::space_info()?;

  return Some((info.index + 1, info.count));
}
