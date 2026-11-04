pub mod junk;
pub mod large_files;

use iced::widget::{button, center, column, text};
use iced::{Center, Element};

pub fn prompt<'a, Message: Clone + 'a>(
    title: &'static str,
    status: &'static str,
    on_scan: Option<Message>,
) -> Element<'a, Message> {
    let scan = button(text("Scan"))
        .padding([10, 24])
        .on_press_maybe(on_scan);

    center(
        column![text(title).size(28), text(status), scan]
            .spacing(16)
            .align_x(Center),
    )
    .into()
}
