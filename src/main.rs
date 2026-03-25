use std::sync::LazyLock;

use camino::Utf8PathBuf;
use clap::Parser;
use color_eyre::eyre::{ContextCompat as _, Result};
use etcetera::BaseStrategy;
use figment2::{
  Figment,
  providers::{Env, Format as _, Toml},
};
use objc2::{MainThreadMarker, runtime::ProtocolObject};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

use crate::{config::Config, constants::SWOOSHER_CONFIG_PREFIX, delegate::AppDelegate, watcher::watch_config};

mod config;
mod constants;
mod delegate;
mod launch_agent;
mod server;
mod spaces;
mod ui;
mod updater;
mod utils;
mod watcher;

#[derive(Parser)]
#[command()]
struct CliArgs {
  /// Path to the Unix domain socket.
  #[arg(long)]
  socket_path: Option<Utf8PathBuf>,

  /// Open the configuration file in the default text editor.
  #[arg(long)]
  open_config: bool,

  /// Open the logs directory in the default file manager.
  #[arg(long)]
  open_logs: bool,
}

pub static CONFIG_PATH: LazyLock<Utf8PathBuf> = LazyLock::new(|| {
  let config_dir = etcetera::base_strategy::Xdg::new()
    .ok()
    .and_then(|s| Utf8PathBuf::try_from(etcetera::BaseStrategy::config_dir(&s)).ok())
    .unwrap_or_else(|| Utf8PathBuf::from("~/.config"));

  return config_dir.join("swoosher").join("config.toml");
});

static SOCKET_PATH: LazyLock<Utf8PathBuf> = LazyLock::new(|| {
  let strategy = etcetera::choose_base_strategy() //
    .expect("failed to determine state directory");

  let base = strategy.state_dir().unwrap_or_else(|| etcetera::BaseStrategy::data_dir(&strategy));

  return Utf8PathBuf::try_from(base)
    .expect("state directory is not valid UTF-8")
    .join("swoosher")
    .join("daemon.sock");
});

fn main() -> Result<()> {
  let args = CliArgs::parse();

  color_eyre::install()?;
  utils::log::init_logger();

  Config::ensure_exists()?;

  if args.open_config {
    edit::edit_file(&*CONFIG_PATH)?;
    return Ok(());
  }

  if args.open_logs {
    open::that(&*utils::log::LOG_DIR)?;
    return Ok(());
  }

  // Load configuration.
  let config: Config = Figment::new()
    .merge(Toml::file(&*CONFIG_PATH))
    .merge(Env::prefixed(SWOOSHER_CONFIG_PREFIX).split("_"))
    .extract()?;

  // Initialize the CGEvent tap for posting privileges.
  spaces::init_event_tap()?;

  let socket_path = args.socket_path.unwrap_or_else(|| SOCKET_PATH.clone());

  // Run the socket server on a background thread.
  let server = server::Server::bind(&socket_path, config.timeout)?;
  std::thread::spawn(move || {
    if let Err(e) = server.run() {
      log::error!("Server failed: {e:#}");
    }
  });

  // Run the macOS app (tray icon) on the main thread.
  let mtm = MainThreadMarker::new().context("Failed to create main thread marker")?;

  let app = NSApplication::sharedApplication(mtm);
  app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

  let delegate = AppDelegate::new(mtm, config);

  // Watch config file for changes.
  let watcher = watch_config(&delegate, mtm).inspect_err(|e| log::warn!("{e:#}")).ok();

  // Run application.
  let delegate = ProtocolObject::from_ref(&*delegate);
  app.setDelegate(Some(delegate));
  app.run();

  // Clean up socket on exit.
  fs_err::remove_file(&socket_path).ok();

  drop(watcher);

  return Ok(());
}
