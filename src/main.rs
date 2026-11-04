mod app;
mod format;
mod recycle;
mod scan;
mod section;
mod task;
mod ui;

use app::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::TITLE)
        .window_size((960.0, 640.0))
        .centered()
        .run()
}
