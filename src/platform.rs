use std::ffi::CStr;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::{ptr, slice};

use iced::widget::image::Handle;
use iced::window::Window;
use iced::window::raw_window_handle::RawWindowHandle;
use objc2::rc::autoreleasepool;
use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSApplication, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
    NSGraphicsContext, NSImage, NSToolbar, NSView, NSWindowToolbarStyle, NSWorkspace,
};
use objc2_foundation::{NSData, NSPoint, NSRect, NSSize, NSString};

const OPEN_FILES: libc::rlim_t = 10_240;
const MAX_SCAN_THREADS: usize = 8;
const ICON_PIXELS: usize = 64;
const APP_ICON: &[u8] = include_bytes!("../assets/icon.png");

pub type FileIcon = Handle;

pub fn icon(path: &Path) -> Option<FileIcon> {
    let path = NSString::from_str(path.to_str()?);
    autoreleasepool(|_| render(&NSWorkspace::sharedWorkspace().iconForFile(&path)))
}

pub fn folder_icon() -> Option<FileIcon> {
    icon(Path::new("/private"))
}

pub fn application(bundle_id: &str) -> Option<PathBuf> {
    let bundle_id = NSString::from_str(bundle_id);

    autoreleasepool(|_| {
        let url =
            NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&bundle_id)?;
        Some(PathBuf::from(url.path()?.to_string()))
    })
}

pub fn reveal(path: &Path) {
    let Some(path) = path.to_str() else {
        return;
    };

    let path = NSString::from_str(path);
    let root = NSString::from_str("");

    autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace().selectFile_inFileViewerRootedAtPath(Some(&path), &root);
    });
}

fn render(image: &NSImage) -> Option<FileIcon> {
    let side = ICON_PIXELS as isize;

    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            ptr::null_mut(),
            side,
            side,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }?;

    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
    let bounds = NSRect::new(
        NSPoint::ZERO,
        NSSize::new(ICON_PIXELS as f64, ICON_PIXELS as f64),
    );

    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    image.drawInRect_fromRect_operation_fraction(
        bounds,
        NSRect::ZERO,
        NSCompositingOperation::Copy,
        1.0,
    );
    context.flushGraphics();
    NSGraphicsContext::restoreGraphicsState_class();

    let data = bitmap.bitmapData();
    let stride = bitmap.bytesPerRow() as usize;
    if data.is_null() || stride < ICON_PIXELS * 4 {
        return None;
    }

    let bytes = unsafe { slice::from_raw_parts(data, stride * ICON_PIXELS) };

    let mut pixels = Vec::with_capacity(ICON_PIXELS * ICON_PIXELS * 4);
    for row in bytes.chunks_exact(stride) {
        for pixel in row[..ICON_PIXELS * 4].chunks_exact(4) {
            let alpha = pixel[3];
            let straight = |channel: u8| match alpha {
                0 => 0,
                _ => (u16::from(channel) * 255 / u16::from(alpha)).min(255) as u8,
            };
            pixels.extend([
                straight(pixel[0]),
                straight(pixel[1]),
                straight(pixel[2]),
                alpha,
            ]);
        }
    }

    Some(Handle::from_rgba(
        ICON_PIXELS as u32,
        ICON_PIXELS as u32,
        pixels,
    ))
}

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

    set_dock_icon(main_thread);
}

fn set_dock_icon(main_thread: MainThreadMarker) {
    let data = NSData::with_bytes(APP_ICON);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        return;
    };

    unsafe { NSApplication::sharedApplication(main_thread).setApplicationIconImage(Some(&image)) };
}
