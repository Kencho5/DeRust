use iced::window::Window;
use iced::window::raw_window_handle::RawWindowHandle;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSToolbar, NSView, NSWindowToolbarStyle};

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
