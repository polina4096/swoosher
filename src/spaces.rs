use std::{ffi::c_void, ptr};

// CoreFoundation / CoreGraphics types.
type CFAllocatorRef = *const c_void;
type CFTypeRef = *const c_void;
type CFArrayRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFStringRef = *const c_void;
type CFNumberRef = *const c_void;
type CFUUIDRef = *const c_void;
type CFIndex = isize;
type CFMachPortRef = *const c_void;
type CFRunLoopSourceRef = *const c_void;
type CFRunLoopRef = *const c_void;
type CFRunLoopMode = CFStringRef;
type CFNumberType = i32;
type CFTypeID = u64;
type CGEventRef = *const c_void;
type CGEventTapProxy = *const c_void;
type CGEventField = u32;
type CGEventMask = u64;
type CGEventType = u32;
type CGDirectDisplayID = u32;
type CGError = i32;

type CGSConnectionID = i32;
type CGSSpaceID = u64;

type CGEventTapCallBack = unsafe extern "C" fn(
  proxy: CGEventTapProxy,
  event_type: CGEventType,
  event: CGEventRef,
  refcon: *mut c_void,
) -> CGEventRef;

const K_CF_NUMBER_SINT64_TYPE: CFNumberType = 4;
const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;
const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_KEY_UP: u32 = 11;
const K_CG_ERROR_SUCCESS: CGError = 0;

// Private CGEvent fields for synthetic dock swipe gestures.
const CG_EVENT_TYPE_FIELD: CGEventField = 55;
const CG_EVENT_GESTURE_HID_TYPE: CGEventField = 110;
const CG_EVENT_GESTURE_SCROLL_Y: CGEventField = 119;
const CG_EVENT_GESTURE_SWIPE_MOTION: CGEventField = 123;
const CG_EVENT_GESTURE_SWIPE_PROGRESS: CGEventField = 124;
const CG_EVENT_GESTURE_SWIPE_VELOCITY_X: CGEventField = 129;
const CG_EVENT_GESTURE_SWIPE_VELOCITY_Y: CGEventField = 130;
const CG_EVENT_GESTURE_PHASE: CGEventField = 132;
const CG_EVENT_SCROLL_GESTURE_FLAG_BITS: CGEventField = 135;
const CG_EVENT_GESTURE_ZOOM_DELTA_X: CGEventField = 139;

const IO_HID_EVENT_TYPE_DOCK_SWIPE: i64 = 23;
const CGS_EVENT_GESTURE: i64 = 29;
const CGS_EVENT_DOCK_CONTROL: i64 = 30;
const CGS_GESTURE_PHASE_BEGAN: i64 = 1;
const CGS_GESTURE_PHASE_ENDED: i64 = 4;
const CG_GESTURE_MOTION_HORIZONTAL: i64 = 1;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
  fn CGEventCreate(source: *const c_void) -> CGEventRef;
  fn CGEventPost(tap: u32, event: CGEventRef);
  fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
  fn CGEventSetDoubleValueField(event: CGEventRef, field: CGEventField, value: f64);
  fn CGEventGetLocation(event: CGEventRef) -> CGPoint;

  fn CGEventTapCreate(
    tap: u32,
    place: u32,
    options: u32,
    events_of_interest: CGEventMask,
    callback: CGEventTapCallBack,
    refcon: *mut c_void,
  ) -> CFMachPortRef;
  fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

  fn CGGetDisplaysWithPoint(
    point: CGPoint,
    max_displays: u32,
    displays: *mut CGDirectDisplayID,
    matching_display_count: *mut u32,
  ) -> CGError;
  fn CGDisplayCreateUUIDFromDisplayID(display: CGDirectDisplayID) -> CFUUIDRef;

  fn CFRelease(cf: CFTypeRef);
  fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
  fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: CFIndex) -> *const c_void;
  fn CFDictionaryGetValue(dict: CFDictionaryRef, key: *const c_void) -> *const c_void;
  fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
  fn CFArrayGetTypeID() -> CFTypeID;
  fn CFDictionaryGetTypeID() -> CFTypeID;
  fn CFNumberGetTypeID() -> CFTypeID;
  fn CFNumberGetValue(number: CFNumberRef, the_type: CFNumberType, value_ptr: *mut c_void) -> bool;
  fn CFUUIDCreateString(alloc: CFAllocatorRef, uuid: CFUUIDRef) -> CFStringRef;
  fn CFMachPortCreateRunLoopSource(alloc: CFAllocatorRef, port: CFMachPortRef, order: CFIndex) -> CFRunLoopSourceRef;
  fn CFRunLoopGetMain() -> CFRunLoopRef;
  fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFRunLoopMode);

  static kCFRunLoopCommonModes: CFRunLoopMode;

  // Private CGS/SLS APIs (SkyLight framework, loaded via ApplicationServices).
  fn CGSMainConnectionID() -> CGSConnectionID;
  fn CGSGetActiveSpace(connection: CGSConnectionID) -> CGSSpaceID;
  fn CGSCopyManagedDisplaySpaces(connection: CGSConnectionID, display: CFStringRef) -> CFArrayRef;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
  x: f64,
  y: f64,
}

// CFSTR equivalent -- create a CFStringRef from a static Rust string.
unsafe fn cfstr(s: &str) -> CFStringRef {
  unsafe extern "C" {
    fn CFStringCreateWithBytes(
      alloc: CFAllocatorRef,
      bytes: *const u8,
      num_bytes: CFIndex,
      encoding: u32,
      is_external: bool,
    ) -> CFStringRef;
  }

  const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
  unsafe { CFStringCreateWithBytes(ptr::null(), s.as_ptr(), s.len() as CFIndex, K_CF_STRING_ENCODING_UTF8, false) }
}

static mut G_TAP: CFMachPortRef = ptr::null();
static mut G_SOURCE: CFRunLoopSourceRef = ptr::null();

unsafe extern "C" fn tap_callback(
  _proxy: CGEventTapProxy,
  _event_type: CGEventType,
  event: CGEventRef,
  _refcon: *mut c_void,
) -> CGEventRef {
  event
}

/// Initialize an event tap to establish CGEvent posting privileges.
pub fn init() -> bool {
  unsafe {
    if !G_TAP.is_null() {
      return true;
    }

    let mask: CGEventMask = (1 << K_CG_EVENT_KEY_DOWN) | (1 << K_CG_EVENT_KEY_UP);
    G_TAP = CGEventTapCreate(
      K_CG_SESSION_EVENT_TAP,
      K_CG_HEAD_INSERT_EVENT_TAP,
      K_CG_EVENT_TAP_OPTION_DEFAULT,
      mask,
      tap_callback,
      ptr::null_mut(),
    );

    if G_TAP.is_null() {
      return false;
    }

    G_SOURCE = CFMachPortCreateRunLoopSource(ptr::null(), G_TAP, 0);
    CFRunLoopAddSource(CFRunLoopGetMain(), G_SOURCE, kCFRunLoopCommonModes);
    CGEventTapEnable(G_TAP, true);

    return true;
  }
}

pub fn init_event_tap() -> color_eyre::eyre::Result<()> {
  if !init() {
    color_eyre::eyre::bail!("Failed to create event tap. Grant Accessibility permission.");
  }

  return Ok(());
}

/// Post a synthetic dock swipe gesture to switch one space.
pub fn post_switch_gesture(direction: Direction) {
  let is_right = matches!(direction, Direction::Right);
  let flag_dir: i64 = if is_right { 1 } else { 0 };
  let progress: f64 = if is_right { 2.0 } else { -2.0 };
  let velocity: f64 = if is_right { 1600.0 } else { -1600.0 };

  unsafe {
    // Begin gesture.
    let a = CGEventCreate(ptr::null());
    let b = CGEventCreate(ptr::null());
    CGEventSetIntegerValueField(a, CG_EVENT_TYPE_FIELD, CGS_EVENT_GESTURE);
    CGEventSetIntegerValueField(b, CG_EVENT_TYPE_FIELD, CGS_EVENT_DOCK_CONTROL);
    CGEventSetIntegerValueField(b, CG_EVENT_GESTURE_HID_TYPE, IO_HID_EVENT_TYPE_DOCK_SWIPE);
    CGEventSetIntegerValueField(b, CG_EVENT_GESTURE_PHASE, CGS_GESTURE_PHASE_BEGAN);
    CGEventSetIntegerValueField(b, CG_EVENT_SCROLL_GESTURE_FLAG_BITS, flag_dir);
    CGEventSetIntegerValueField(b, CG_EVENT_GESTURE_SWIPE_MOTION, CG_GESTURE_MOTION_HORIZONTAL);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_SCROLL_Y, 0.0);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_ZOOM_DELTA_X, 1.0e-45_f64);
    CGEventPost(K_CG_SESSION_EVENT_TAP, b);
    CGEventPost(K_CG_SESSION_EVENT_TAP, a);
    CFRelease(a);
    CFRelease(b);

    // End gesture.
    let a = CGEventCreate(ptr::null());
    let b = CGEventCreate(ptr::null());
    CGEventSetIntegerValueField(a, CG_EVENT_TYPE_FIELD, CGS_EVENT_GESTURE);
    CGEventSetIntegerValueField(b, CG_EVENT_TYPE_FIELD, CGS_EVENT_DOCK_CONTROL);
    CGEventSetIntegerValueField(b, CG_EVENT_GESTURE_HID_TYPE, IO_HID_EVENT_TYPE_DOCK_SWIPE);
    CGEventSetIntegerValueField(b, CG_EVENT_GESTURE_PHASE, CGS_GESTURE_PHASE_ENDED);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_SWIPE_PROGRESS, progress);
    CGEventSetIntegerValueField(b, CG_EVENT_SCROLL_GESTURE_FLAG_BITS, flag_dir);
    CGEventSetIntegerValueField(b, CG_EVENT_GESTURE_SWIPE_MOTION, CG_GESTURE_MOTION_HORIZONTAL);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_SCROLL_Y, 0.0);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_SWIPE_VELOCITY_X, velocity);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_SWIPE_VELOCITY_Y, 0.0);
    CGEventSetDoubleValueField(b, CG_EVENT_GESTURE_ZOOM_DELTA_X, 1.0e-45_f64);
    CGEventPost(K_CG_SESSION_EVENT_TAP, b);
    CGEventPost(K_CG_SESSION_EVENT_TAP, a);
    CFRelease(a);
    CFRelease(b);
  }
}

#[derive(Clone, Copy)]
pub enum Direction {
  Left,
  Right,
}

pub struct SpaceInfo {
  /// 0-based index of the current space.
  pub index: u32,
  /// Total number of spaces.
  pub count: u32,
}

/// Query current space index and count for the display under the cursor.
pub fn space_info() -> Option<SpaceInfo> {
  unsafe {
    let conn = CGSMainConnectionID();
    if conn == 0 {
      return None;
    }

    let active = CGSGetActiveSpace(conn);
    if active == 0 {
      return None;
    }

    // Get display under cursor.
    let tmp = CGEventCreate(ptr::null());
    let cursor = CGEventGetLocation(tmp);
    CFRelease(tmp);

    let mut cursor_display: CGDirectDisplayID = 0;
    let mut display_count: u32 = 0;
    if CGGetDisplaysWithPoint(cursor, 1, &mut cursor_display, &mut display_count) != K_CG_ERROR_SUCCESS
      || display_count == 0
    {
      return None;
    }

    let uuid = CGDisplayCreateUUIDFromDisplayID(cursor_display);
    let display_id = if !uuid.is_null() {
      let s = CFUUIDCreateString(ptr::null(), uuid);
      CFRelease(uuid);
      s
    }
    else {
      ptr::null()
    };

    let mut displays = CGSCopyManagedDisplaySpaces(conn, display_id);
    if displays.is_null() && !display_id.is_null() {
      displays = CGSCopyManagedDisplaySpaces(conn, ptr::null());
    }
    if !display_id.is_null() {
      CFRelease(display_id);
    }
    if displays.is_null() {
      return None;
    }

    // Find the display dict matching the active space.
    let n = CFArrayGetCount(displays);
    let mut target: CFDictionaryRef = ptr::null();

    let key_current_space = cfstr("Current Space");
    let key_id64 = cfstr("id64");
    let key_spaces = cfstr("Spaces");

    for i in 0 .. n {
      let d = CFArrayGetValueAtIndex(displays, i);
      if target.is_null() {
        target = d;
      }

      let cs = CFDictionaryGetValue(d, key_current_space);
      if !cs.is_null() && CFGetTypeID(cs) == CFDictionaryGetTypeID() {
        let id_num = CFDictionaryGetValue(cs, key_id64);
        if !id_num.is_null() && CFGetTypeID(id_num) == CFNumberGetTypeID() {
          let mut sid: CGSSpaceID = 0;
          CFNumberGetValue(id_num, K_CF_NUMBER_SINT64_TYPE, &mut sid as *mut _ as *mut c_void);
          if sid == active {
            target = d;
            break;
          }
        }
      }
    }

    CFRelease(key_current_space);
    CFRelease(key_id64);

    if target.is_null() {
      CFRelease(key_spaces);
      CFRelease(displays);
      return None;
    }

    let spaces = CFDictionaryGetValue(target, key_spaces);
    CFRelease(key_spaces);

    if spaces.is_null() || CFGetTypeID(spaces) != CFArrayGetTypeID() {
      CFRelease(displays);
      return None;
    }

    let space_count = CFArrayGetCount(spaces);
    let mut total: u32 = 0;
    let mut idx: u32 = 0;
    let mut found = false;

    let key_id64 = cfstr("id64");
    for i in 0 .. space_count {
      let sd = CFArrayGetValueAtIndex(spaces, i);
      let id_num = CFDictionaryGetValue(sd, key_id64);
      if id_num.is_null() {
        continue;
      }
      let mut sid: CGSSpaceID = 0;
      CFNumberGetValue(id_num, K_CF_NUMBER_SINT64_TYPE, &mut sid as *mut _ as *mut c_void);
      if !found && sid == active {
        idx = total;
        found = true;
      }
      total += 1;
    }

    CFRelease(key_id64);
    CFRelease(displays);

    if !found || total == 0 {
      return None;
    }

    return Some(SpaceInfo { index: idx, count: total });
  }
}
