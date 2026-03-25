use objc2::{DefinedClass, MainThreadMarker, rc::Retained, runtime::AnyObject, sel};
use objc2_app_kit::{NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::{NSAttributedString, NSString};

use crate::{delegate::AppDelegate, server, updater::UpdateState};

const SWITCH_TO_ITEM_TAG: isize = 8000;

pub fn build_menu(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenu> {
  let menu = NSMenu::new(mtm);

  // Set app as the menu delegate so menuNeedsUpdate: fires on open.
  let delegate = objc2::runtime::ProtocolObject::from_ref(app);
  menu.setDelegate(Some(delegate));

  menu.addItem(&about_item(mtm, app));
  menu.addItem(&NSMenuItem::separatorItem(mtm));
  menu.addItem(&switch_left_item(mtm, app));
  menu.addItem(&switch_right_item(mtm, app));
  menu.addItem(&switch_to_item(mtm));
  menu.addItem(&NSMenuItem::separatorItem(mtm));
  menu.addItem(&autostart_item(mtm, app));
  menu.addItem(&update_item(mtm, app));
  menu.addItem(&NSMenuItem::separatorItem(mtm));
  menu.addItem(&open_config_item(mtm, app));
  menu.addItem(&open_logs_item(mtm, app));
  menu.addItem(&quit_item(mtm, app));

  return menu;
}

pub fn refresh_switch_to_submenu(menu: &NSMenu, mtm: MainThreadMarker, app: &AppDelegate) {
  let Some(item) = menu.itemWithTag(SWITCH_TO_ITEM_TAG)
  else {
    return;
  };

  let submenu = NSMenu::new(mtm);
  let (current, count) = server::get_space_info().unwrap_or((1, 10));

  for i in 1 ..= count {
    let sub_item = unsafe {
      NSMenuItem::initWithTitle_action_keyEquivalent(
        mtm.alloc::<NSMenuItem>(),
        &NSString::from_str(&format!("Space {i}")),
        Some(sel!(onSwitchToSpace:)),
        &NSString::new(),
      )
    };

    unsafe { sub_item.setTarget(Some(app)) };
    sub_item.setTag(i as isize);

    if i == current {
      sub_item.setState(NSControlStateValueOn);
    }

    submenu.addItem(&sub_item);
  }

  item.setSubmenu(Some(&submenu));
}

fn about_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("About swoosher"),
      Some(sel!(onAbout:)),
      &NSString::new(),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  return item;
}

fn switch_left_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("Switch Left"),
      Some(sel!(onSwitchLeft:)),
      &NSString::new(),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  return item;
}

fn switch_right_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("Switch Right"),
      Some(sel!(onSwitchRight:)),
      &NSString::new(),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  return item;
}

fn switch_to_item(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
  let item = NSMenuItem::new(mtm);

  item.setTitle(&NSString::from_str("Switch to\u{2026}"));
  item.setTag(SWITCH_TO_ITEM_TAG);
  item.setSubmenu(Some(&NSMenu::new(mtm)));

  return item;
}

fn autostart_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("Autostart"),
      Some(sel!(onToggleAutostart:)),
      &NSString::new(),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  if *app.ivars().autostart_enabled.borrow() {
    item.setState(NSControlStateValueOn);
  }

  return item;
}

fn update_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let update_state = app.ivars().updater.state();
  let (title, action, enabled) = match &*update_state {
    UpdateState::Unchecked | UpdateState::UpToDate => {
      ("Check for Updates".to_string(), Some(sel!(onCheckForUpdates:)), true)
    }
    UpdateState::Failed { error } => {
      log::debug!("Previous update check failed: {error}");
      ("Check for Updates".to_string(), Some(sel!(onCheckForUpdates:)), true)
    }
    UpdateState::Available { version, .. } => {
      (format!("Update to v{version}\u{2026}"), Some(sel!(onInstallUpdate:)), true)
    }
    UpdateState::Downloading => ("Downloading Update\u{2026}".to_string(), None, false),
  };

  drop(update_state);

  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str(&title),
      action,
      &NSString::from_str("u"),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  item.setEnabled(enabled);

  return item;
}

fn open_config_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("Open Config\u{2026}"),
      Some(sel!(onOpenConfig:)),
      &NSString::from_str(","),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  return item;
}

fn open_logs_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("Open Logs\u{2026}"),
      Some(sel!(onOpenLogs:)),
      &NSString::from_str("l"),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  return item;
}

fn quit_item(mtm: MainThreadMarker, app: &AppDelegate) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      mtm.alloc::<NSMenuItem>(),
      &NSString::from_str("Quit"),
      Some(sel!(onQuit:)),
      &NSString::from_str("q"),
    )
  };

  unsafe { item.setTarget(Some(app)) };

  return item;
}

pub fn build_credits() -> Retained<NSAttributedString> {
  let mtm = MainThreadMarker::new().expect("Must be on main thread");

  let html = concat!(
    r#"<div style="text-align: center; font-family: -apple-system; font-size: 11px; color: #888;">"#,
    "Instant space switcher for macOS.<br/>",
    r#"<a href="https://github.com/polina4096/swoosher/issues">Issues</a>"#,
    " &bull; ",
    r#"<a href="https://github.com/polina4096/swoosher">Source Code</a>"#,
    "</div>"
  );

  let ns_html = NSString::from_str(html);
  let ns_data: Retained<AnyObject> = unsafe { objc2::msg_send![&ns_html, dataUsingEncoding: 4_usize] };

  let doc_type_key = NSString::from_str("DocumentType");
  let doc_type_val = NSString::from_str("NSHTML");
  let opts: Retained<AnyObject> = unsafe {
    objc2::msg_send![
      objc2::class!(NSDictionary),
      dictionaryWithObject: &*doc_type_val,
      forKey: &*doc_type_key
    ]
  };

  let mut doc_attrs: *mut AnyObject = std::ptr::null_mut();
  let result: Retained<NSAttributedString> = unsafe {
    objc2::msg_send![
      mtm.alloc::<NSAttributedString>(),
      initWithData: &*ns_data,
      options: &*opts,
      documentAttributes: &mut doc_attrs,
      error: std::ptr::null_mut::<*mut AnyObject>()
    ]
  };

  return result;
}
