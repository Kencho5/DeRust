use std::ffi::CStr;
use std::mem::size_of;

use iced::window::Window;
use iced::window::raw_window_handle::RawWindowHandle;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSToolbar, NSView, NSWindowToolbarStyle};

const OPEN_FILES: libc::rlim_t = 10_240;
const MAX_SCAN_THREADS: usize = 8;

pub fn configure_threads() {
    let performance_cores = sysctl(c"hw.perflevel0.physicalcpu").unwrap_or_else(|| {
        std::thread::available_parallelism().map_or(MAX_SCAN_THREADS, |count| count.get() / 2)
    });

    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(performance_cores.clamp(2, MAX_SCAN_THREADS))
        .build_global();
}

fn sysctl(name: &CStr) -> Option<usize> {
    let mut value: u32 = 0;
    let mut size = size_of::<u32>();

    let status = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&raw mut value).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };

    (status == 0 && value > 0).then_some(value as usize)
}

pub fn raise_open_file_limit() {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return;
    }

    let target = OPEN_FILES.min(limit.rlim_max);
    if limit.rlim_cur < target {
        limit.rlim_cur = target;
        unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) };
    }
}

pub fn unify_titlebar(window: &dyn Window) {
    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };

    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    let Some(window) = view.window() else {
        return;
    };

    let toolbar = NSToolbar::new(main_thread);
    window.setToolbar(Some(&toolbar));
    window.setToolbarStyle(NSWindowToolbarStyle::Unified);
}
