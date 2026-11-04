mod app;
mod format;
mod platform;
mod recycle;
mod scan;
mod section;
mod task;
mod ui;

use iced::Size;
use iced::window::{self, settings::PlatformSpecific};

use app::App;
use ui::style;

fn main() -> iced::Result {
    platform::raise_open_file_limit();
    platform::configure_threads();

    style::FONTS
        .into_iter()
        .fold(
            iced::application(App::new, App::update, App::view),
            |application, font| application.font(font),
        )
        .title(App::TITLE)
        .theme(App::theme)
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .default_font(style::FONT)
        .window(window::Settings {
            size: Size::new(1100.0, 720.0),
            min_size: Some(Size::new(900.0, 560.0)),
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
