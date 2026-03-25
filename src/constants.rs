/// Disable all logging (stderr and disk).
pub const SWOOSHER_NO_LOGS: &str = "SWOOSHER_NO_LOGS";

/// Disable disk logging (stderr only).
pub const SWOOSHER_NO_DISK_LOGS: &str = "SWOOSHER_NO_DISK_LOGS";

/// Override the log directory path.
pub const SWOOSHER_OVERRIDE_LOG_DIR: &str = "SWOOSHER_OVERRIDE_LOG_DIR";

/// Override the current version string for update checking.
pub const SWOOSHER_OVERRIDE_VERSION: &str = "SWOOSHER_OVERRIDE_VERSION";

/// Prefix for environment variable config overrides (e.g. `SWOOSHER_CONFIG_TIMEOUT=60`).
pub const SWOOSHER_CONFIG_PREFIX: &str = "SWOOSHER_CONFIG_";
