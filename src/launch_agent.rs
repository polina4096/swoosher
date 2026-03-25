use camino::Utf8PathBuf;

const LABEL: &str = "fish.stupid.swoosher";

fn plist_path() -> Utf8PathBuf {
  let home = std::env::var("HOME").expect("HOME not set");
  Utf8PathBuf::from(home).join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

pub fn installed() -> bool {
  plist_path().exists()
}

pub fn install() {
  let exe = match std::env::current_exe() {
    Ok(p) => p,
    Err(e) => {
      log::error!("Failed to get current exe: {e}");
      return;
    }
  };

  let plist = format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
</dict>
</plist>
"#,
    exe = exe.display()
  );

  let path = plist_path();

  if let Some(parent) = path.parent() {
    fs_err::create_dir_all(parent).ok();
  }

  if let Err(e) = fs_err::write(&path, plist) {
    log::error!("Failed to write launch agent: {e}");
    return;
  }

  log::info!("Installed launch agent at {path} (active on next login)");
}

pub fn remove() {
  let path = plist_path();

  if let Err(e) = fs_err::remove_file(&path) {
    log::error!("Failed to remove launch agent: {e}");
    return;
  }

  log::info!("Removed launch agent at {path}");
}
