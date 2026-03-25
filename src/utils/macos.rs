macro_rules! schedule_timer {
  ($interval:expr, $target:expr, $selector:ident) => {{
    let timer = unsafe {
      objc2_foundation::NSTimer::timerWithTimeInterval_target_selector_userInfo_repeats(
        $interval,
        $target,
        objc2::sel!($selector:),
        None,
        true,
      )
    };

    unsafe {
      objc2_foundation::NSRunLoop::currentRunLoop()
        .addTimer_forMode(&timer, objc2_foundation::NSDefaultRunLoopMode);
    }
  }};
}

pub(crate) use schedule_timer;
