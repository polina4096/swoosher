use color_eyre::eyre::ContextCompat as _;
use documented::DocumentedFields;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use toml_edit::DocumentMut;

use crate::CONFIG_PATH;

#[derive(SmartDefault, Deserialize, Serialize, DocumentedFields)]
#[serde(default)]
pub struct Config {
  /// Whether to automatically check for updates on startup.
  #[default = true]
  pub check_updates: bool,

  /// Whether to automatically install updates when available.
  pub auto_update: bool,

  /// Connection read timeout in seconds. Idle connections are closed after this duration. Set to 0 to disable.
  #[default = 30]
  pub timeout: u64,
}

impl Config {
  pub fn default_toml() -> color_eyre::eyre::Result<String> {
    let toml_str = toml_edit::ser::to_string_pretty(&Self::default())?;
    let mut doc: DocumentMut = toml_str.parse()?;

    for (i, (key, comment)) in doc.clone().iter().zip(Self::FIELD_DOCS.iter()).enumerate() {
      let prefix = if i == 0 { format!("# {comment}\n") } else { format!("\n# {comment}\n") };
      let item = doc.get_mut(key.0).context("missing key in serialized config")?;

      if let Some(table) = item.as_table_mut() {
        table.decor_mut().set_prefix(prefix);
      }
      else {
        let mut key = doc.key_mut(key.0).context("missing key in serialized config")?;
        key.leaf_decor_mut().set_prefix(prefix);
      }
    }

    return Ok(doc.to_string());
  }

  pub fn ensure_exists() -> color_eyre::eyre::Result<()> {
    let config_path = &*CONFIG_PATH;

    if !config_path.exists() {
      if let Some(parent) = config_path.parent() {
        fs_err::create_dir_all(parent)?;
      }

      fs_err::write(config_path, Config::default_toml()?)?;
    }

    return Ok(());
  }
}
