pub mod junk;
pub mod large_files;
pub mod style;
pub mod uninstaller;

use iced::widget::text::IntoFragment;
use iced::widget::{
    Column, Text, button, center, checkbox, column, container, progress_bar, row, rule, scrollable,
    space, text,
};
use iced::{Center, Element, Fill, Padding, Theme};
use lucide_icons::Icon;

use crate::format;
use crate::scan::progress::Snapshot;

pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon.unicode()).font(style::ICONS)
}

pub fn prompt<'a, Message: Clone + 'a>(
    icon: Icon,
    title: &'static str,
    status: &'static str,
    on_scan: Option<Message>,
) -> Element<'a, Message> {
    let scan = button(center(text("Scan").size(22).font(style::BOLD)))
        .width(140)
        .height(140)
        .style(style::scan)
        .on_press_maybe(on_scan);

    center(
        column![
            self::icon(icon).size(72).style(style::folder),
            space().height(8),
            text(title).size(34).font(style::BOLD),
            text(status).size(15).style(style::muted),
            space().height(32),
            scan,
        ]
        .spacing(8)
        .align_x(Center),
    )
    .into()
}

pub fn scanning<'a, Message: 'a>(
    icon: Icon,
    title: &'static str,
    status: &'static str,
    snapshot: Snapshot,
) -> Element<'a, Message> {
    let mut stats = column![
        text(format!("{} files", format::count(snapshot.files)))
            .size(26)
            .font(style::BOLD),
        text(format::bytes(snapshot.bytes))
            .size(15)
            .style(style::muted),
    ]
    .spacing(6)
    .align_x(Center);

    if let Some(fraction) = snapshot.fraction() {
        stats = stats.push(space().height(10)).push(
            progress_bar(0.0..=1.0, fraction)
                .length(320)
                .girth(6)
                .style(style::progress),
        );
    }

    center(
        column![
            self::icon(icon).size(72).style(style::folder),
            space().height(8),
            text(title).size(34).font(style::BOLD),
            text(status).size(15).style(style::muted),
            space().height(32),
            container(stats).center_y(140),
        ]
        .spacing(8)
        .align_x(Center),
    )
    .into()
}

pub fn placeholder<'a, Message: 'a>(icon: Icon, title: &'static str) -> Element<'a, Message> {
    center(
        column![
            self::icon(icon).size(72).style(style::folder),
            text(title).size(34).font(style::BOLD),
            text("Coming soon").size(15).style(style::muted),
        ]
        .spacing(12)
        .align_x(Center),
    )
    .into()
}

pub fn page<'a, Message: 'a>(
    icon: Icon,
    title: &'static str,
    subtitle: String,
    notice: Option<String>,
    rows: impl IntoIterator<Item = Element<'a, Message>>,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut heading = column![
        text(title).size(22).font(style::BOLD),
        text(subtitle).size(14).style(style::muted),
    ]
    .spacing(4);

    if let Some(notice) = notice {
        heading = heading.push(text(notice).size(13).style(style::notice));
    }

    let header = row![self::icon(icon).size(40).style(style::folder), heading]
        .spacing(16)
        .align_y(Center);

    let list = Column::with_children(rows).spacing(4).padding(Padding {
        right: 12.0,
        ..Padding::ZERO
    });

    let panel =
        container(column![header, rule::horizontal(1), scrollable(list).height(Fill)].spacing(16))
            .padding(24)
            .height(Fill)
            .style(style::panel);

    column![panel, footer].spacing(16).into()
}

pub fn item<'a, Message: 'a>(
    checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
    icon: Icon,
    name: impl IntoFragment<'a>,
    details: String,
    bytes: u64,
) -> Element<'a, Message> {
    container(
        row![
            checkbox(checked).on_toggle(on_toggle).style(style::check),
            self::icon(icon).size(26).style(style::folder),
            column![
                text(name).size(15),
                text(details).size(12).style(style::muted)
            ]
            .spacing(2),
            space().width(Fill),
            text(format::bytes(bytes)).size(14).style(style::subtle),
        ]
        .spacing(14)
        .align_y(Center),
    )
    .padding([8, 4])
    .into()
}

pub fn footer<'a, Message: 'a>(
    summary: String,
    actions: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut bar = row![
        icon(Icon::Info).size(16).style(style::muted),
        text(summary).size(15),
        space().width(Fill),
    ]
    .spacing(10)
    .align_y(Center);

    for action in actions {
        bar = bar.push(action);
    }

    bar.into()
}

pub fn selection(count: usize, bytes: u64) -> String {
    match count {
        0 => "No items selected".to_owned(),
        1 => format!("1 item selected  |  {}", format::bytes(bytes)),
        count => format!("{count} items selected  |  {}", format::bytes(bytes)),
    }
}

pub fn action<'a, Message: Clone + 'a>(
    label: &'static str,
    style: fn(&Theme, button::Status) -> button::Style,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(text(label).size(15).font(style::BOLD))
        .padding([12, 28])
        .style(style)
        .on_press_maybe(on_press)
        .into()
}
