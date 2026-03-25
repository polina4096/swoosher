use std::cell::RefCell;

use dispatch2::DispatchQueue;
use objc2::{
  DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
  rc::Retained,
  runtime::{AnyObject, NSObject},
};
use objc2_app_kit::{
  NSAboutPanelOptionApplicationName, NSAboutPanelOptionApplicationVersion, NSAboutPanelOptionCredits,
  NSAboutPanelOptionVersion, NSApplication, NSApplicationDelegate, NSMenu, NSMenuDelegate, NSStatusBar, NSStatusItem,
  NSVariableStatusItemLength,
};
use objc2_foundation::{NSDictionary, NSNotification, NSObjectProtocol, NSString, NSTimer};

use crate::{
  config::Config,
  launch_agent, server,
  ui::views,
  updater::{self, UpdateState, Updater},
};

pub struct AppDelegateIvars {
  status_item: RefCell<Option<Retained<NSStatusItem>>>,
  pub updater: Updater,
  pub autostart_enabled: RefCell<bool>,
  config: RefCell<Config>,
}

define_class!(
  #[unsafe(super(NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "AppDelegate"]
  #[ivars = AppDelegateIvars]
  pub struct AppDelegate;

  impl AppDelegate {
    #[unsafe(method(onQuit:))]
    fn on_quit(&self, _sender: &AnyObject) {
      NSApplication::sharedApplication(self.mtm()).terminate(None);
    }

    #[unsafe(method(onOpenConfig:))]
    fn on_open_config(&self, _sender: &AnyObject) {
      if let Err(e) = open::that(&*crate::CONFIG_PATH) {
        log::error!("Failed to open config file: {e}");
      }
    }

    #[unsafe(method(onOpenLogs:))]
    fn on_open_logs(&self, _sender: &AnyObject) {
      if let Err(e) = open::that(&*crate::utils::log::LOG_DIR) {
        log::error!("Failed to open logs directory: {e}");
      }
    }

    #[unsafe(method(onCheckForUpdates:))]
    fn on_check_for_updates(&self, _sender: &AnyObject) {
      self.attempt_update();
    }

    #[unsafe(method(onInstallUpdate:))]
    fn on_install_update(&self, _sender: &AnyObject) {
      self.install_update();
    }

    #[unsafe(method(onToggleAutostart:))]
    fn on_toggle_autostart(&self, _sender: &AnyObject) {
      let current = *self.ivars().autostart_enabled.borrow();
      if current {
        launch_agent::remove();
      }
      else {
        launch_agent::install();
      }

      *self.ivars().autostart_enabled.borrow_mut() = !current;
      self.rebuild_menu();
    }

    #[unsafe(method(onSwitchLeft:))]
    fn on_switch_left(&self, _sender: &AnyObject) {
      server::switch_left();
    }

    #[unsafe(method(onSwitchRight:))]
    fn on_switch_right(&self, _sender: &AnyObject) {
      server::switch_right();
    }

    #[unsafe(method(onSwitchToSpace:))]
    fn on_switch_to_space(&self, sender: &AnyObject) {
      let tag: isize = unsafe { msg_send![sender, tag] };
      server::switch_to(tag as u32);
    }

    #[unsafe(method(onAbout:))]
    fn on_about(&self, _sender: &AnyObject) {
      let mtm = self.mtm();
      let app = NSApplication::sharedApplication(mtm);

      let name = NSString::from_str("swoosher");
      let app_version = NSString::from_str(env!("CARGO_PKG_VERSION"));
      let build_version = NSString::from_str(env!("GIT_COMMIT_SHORT"));
      let credits = views::build_credits();

      let keys: &[&NSString] = &[
        unsafe { NSAboutPanelOptionApplicationName },
        unsafe { NSAboutPanelOptionApplicationVersion },
        unsafe { NSAboutPanelOptionVersion },
        unsafe { NSAboutPanelOptionCredits },
      ];
      let values: &[&AnyObject] = &[&name, &app_version, &build_version, &credits];
      let options = NSDictionary::from_slices(keys, values);

      #[allow(deprecated)]
      app.activateIgnoringOtherApps(true);

      unsafe { app.orderFrontStandardAboutPanelWithOptions(&options) };
    }

    #[unsafe(method(onTimer:))]
    fn on_timer(&self, _timer: &NSTimer) {
      if self.config().check_updates {
        self.attempt_update();
      }
    }
  }

  unsafe impl NSObjectProtocol for AppDelegate {}

  unsafe impl NSApplicationDelegate for AppDelegate {
    #[unsafe(method(applicationDidFinishLaunching:))]
    fn did_finish_launching(&self, _notification: &NSNotification) {
      let mtm = self.mtm();

      let status_bar = NSStatusBar::systemStatusBar();
      let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

      if let Some(button) = status_item.button(mtm) {
        button.setTitle(&NSString::from_str("SSS"));
      }

      *self.ivars().status_item.borrow_mut() = Some(status_item);

      self.rebuild_menu();

      if self.config().check_updates {
        self.attempt_update();
      }

      // Periodic update check (every 6 hours).
      crate::utils::macos::schedule_timer!(21600.0, self, onTimer);
    }
  }

  unsafe impl NSMenuDelegate for AppDelegate {
    #[unsafe(method(menuNeedsUpdate:))]
    fn menu_needs_update(&self, menu: &NSMenu) {
      views::refresh_switch_to_submenu(menu, self.mtm(), self);
    }
  }
);

impl AppDelegate {
  pub fn new(mtm: MainThreadMarker, config: Config) -> Retained<Self> {
    let autostart_enabled = launch_agent::installed();

    let this = mtm.alloc::<AppDelegate>();
    let this = this.set_ivars(AppDelegateIvars {
      status_item: RefCell::new(None),
      updater: Updater::new(),
      autostart_enabled: RefCell::new(autostart_enabled),
      config: RefCell::new(config),
    });
    let this: Retained<Self> = unsafe { msg_send![super(this), init] };

    return this;
  }

  pub fn config(&self) -> std::cell::Ref<'_, Config> {
    return self.ivars().config.borrow();
  }

  pub fn reload_config(&self, new_config: Config) {
    log::info!("Config reloaded");
    *self.ivars().config.borrow_mut() = new_config;
    self.rebuild_menu();
  }

  fn rebuild_menu(&self) {
    let mtm = self.mtm();
    let menu = views::build_menu(mtm, self);

    if let Some(status_item) = self.ivars().status_item.borrow().as_ref() {
      status_item.setMenu(Some(&menu));
    }
  }

  fn attempt_update(&self) {
    let mtm = self.mtm();
    let this = dispatch2::MainThreadBound::new(self.retain(), mtm);

    std::thread::spawn(move || {
      let new_state = updater::check_for_update();

      DispatchQueue::main().exec_async(move || {
        let mtm = MainThreadMarker::new().expect("Must be on main thread");
        let delegate = this.get(mtm);
        let is_available = matches!(&new_state, UpdateState::Available { .. });
        let should_auto_install = is_available && delegate.config().auto_update;

        delegate.ivars().updater.set_state(new_state);
        delegate.rebuild_menu();

        if should_auto_install {
          delegate.install_update();
        }
      });
    });
  }

  fn install_update(&self) {
    let state = self.ivars().updater.state();
    let UpdateState::Available { download_url, .. } = &*state
    else {
      return;
    };
    let url = download_url.clone();
    drop(state);

    self.ivars().updater.set_state(UpdateState::Downloading);
    self.rebuild_menu();

    let mtm = self.mtm();
    let this = dispatch2::MainThreadBound::new(self.retain(), mtm);

    std::thread::spawn(move || {
      match updater::download_and_install(&url) {
        Ok(()) => {}
        Err(e) => {
          log::error!("Update install failed: {e:#}");

          DispatchQueue::main().exec_async(move || {
            let mtm = MainThreadMarker::new().expect("Must be on main thread");
            let delegate = this.get(mtm);
            delegate.ivars().updater.set_state(UpdateState::Failed { error: e.to_string() });
            delegate.rebuild_menu();
          });
        }
      }
    });
  }
}
