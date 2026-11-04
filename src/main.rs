mod app;
mod format;
mod recycle;
mod scan;
mod section;
mod task;
mod ui;

use iced::Size;
use iced::window::{self, settings::PlatformSpecific};

use app::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::TITLE)
        .theme(App::theme)
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .window(window::Settings {
            size: Size::new(960.0, 640.0),
            min_size: Some(Size::new(760.0, 480.0)),
            position: window::Position::Centered,
            platform_specific: PlatformSpecific {
                title_hidden: true,
                titlebar_transparent: true,
                fullsize_content_view: true,
            },
            ..window::Settings::default()
        })
        .run()
}
