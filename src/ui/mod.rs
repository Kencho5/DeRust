pub mod explorer;
pub mod junk;
pub mod overview;
pub mod style;
pub mod treemap;
pub mod uninstaller;

use std::borrow::Cow;

use iced::alignment::Horizontal;
use iced::widget::text::{IntoFragment, Wrapping};
use iced::widget::{
    Column, Row, Text, button, center, checkbox, column, container, progress_bar, row, rule,
    scrollable, space, text,
};
use iced::{Center, Element, Fill, Length, Theme};
use lucide_icons::Icon;

use crate::format;
use crate::scan::progress::Snapshot;
use style::Tone;

pub const CHECK_WIDTH: f32 = 28.0;

const MAX_NAME_CHARS: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sort<Key> {
    pub key: Key,
    pub descending: bool,
}

impl<Key: PartialEq> Sort<Key> {
    pub fn toggle(self, key: Key, descending_first: bool) -> Self {
        if self.key == key {
            Sort {
                key,
                descending: !self.descending,
            }
        } else {
            Sort {
                key,
                descending: descending_first,
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Summary {
    Idle,
    Scanning(Snapshot),
    Busy,
    Found { items: u64, bytes: u64 },
}

pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon.unicode()).font(style::ICONS)
}

pub fn page<'a, Message: 'a>(
    title: &'static str,
    subtitle: &'static str,
    actions: Option<Element<'a, Message>>,
    body: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut header = row![
        column![
            text(title)
                .size(28)
                .font(style::SEMIBOLD)
                .style(style::strong),
            text(subtitle).size(14).style(style::muted),
        ]
        .spacing(4),
        space().width(Fill),
    ]
    .align_y(Center);

    if let Some(actions) = actions {
        header = header.push(actions);
    }

    let mut page = column![header, space().height(8)].spacing(16);
    for element in body {
        page = page.push(element);
    }

    page.into()
}

pub fn empty<'a, Message: Clone + 'a>(
    icon: Icon,
    title: &'static str,
    description: &'static str,
    action: Element<'a, Message>,
) -> Element<'a, Message> {
    container(center(
        column![
            self::icon(icon).size(28).style(style::muted),
            space().height(4),
            text(title)
                .size(16)
                .font(style::SEMIBOLD)
                .style(style::strong),
            text(description).size(14).style(style::muted),
            space().height(8),
            action,
        ]
        .spacing(6)
        .align_x(Center),
    ))
    .height(Fill)
    .style(style::card)
    .into()
}

pub fn progress<'a, Message: 'a>(label: &'static str, snapshot: Snapshot) -> Element<'a, Message> {
    let mut content = column![
        row![
            text(label)
                .size(14)
                .font(style::MEDIUM)
                .style(style::strong),
            space().width(Fill),
            text(format!(
                "{} files  ·  {}",
                format::count(snapshot.files),
                format::bytes(snapshot.bytes)
            ))
            .size(14)
            .style(style::muted),
        ]
        .align_y(Center),
    ]
    .spacing(12);

    if let Some(fraction) = snapshot.fraction() {
        content = content.push(
            progress_bar(0.0..=1.0, fraction)
                .girth(4)
                .style(style::progress),
        );
    }

    container(content.push(metrics(snapshot)))
        .padding(16)
        .width(Fill)
        .style(style::card)
        .into()
}

pub fn with_metrics<'a, Message: 'a>(
    snapshot: Option<Snapshot>,
    action: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut actions = row![].spacing(24).align_y(Center);

    if let Some(snapshot) = snapshot {
        actions = actions.push(metrics(snapshot));
    }

    actions.push(action).into()
}

fn metrics<'a, Message: 'a>(snapshot: Snapshot) -> Element<'a, Message> {
    let metrics = [
        (Icon::Timer, format::duration(snapshot.elapsed)),
        (
            Icon::Gauge,
            format!("{} files/s", format::count(snapshot.files_per_second())),
        ),
        (
            Icon::Activity,
            format!("{}/s", format::bytes(snapshot.bytes_per_second())),
        ),
        (
            Icon::Cpu,
            format!("{} threads", rayon::current_num_threads()),
        ),
    ];

    Row::with_children(metrics.into_iter().map(|(icon, value)| {
        row![
            self::icon(icon).size(14).style(style::muted),
            text(value).size(13).style(style::muted),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    }))
    .spacing(20)
    .into()
}

pub fn total<'a, Message: 'a>(
    value: String,
    label: &'static str,
    caption: String,
) -> Column<'a, Message> {
    column![
        row![
            text(value)
                .size(22)
                .font(style::SEMIBOLD)
                .style(style::strong),
            text(label).size(14).style(style::muted),
        ]
        .spacing(8)
        .align_y(Center),
        text(caption).size(14).style(style::muted),
    ]
    .spacing(4)
}

pub fn segments<'a, Message: 'a>(
    parts: impl IntoIterator<Item = (u64, Option<Tone>)>,
) -> Element<'a, Message> {
    bar(parts, 6.0)
}

pub fn bar<'a, Message: 'a>(
    parts: impl IntoIterator<Item = (u64, Option<Tone>)>,
    height: f32,
) -> Element<'a, Message> {
    let parts: Vec<(u64, Option<Tone>)> =
        parts.into_iter().filter(|&(bytes, _)| bytes > 0).collect();
    let total: u64 = parts.iter().map(|&(bytes, _)| bytes).sum();

    if total == 0 {
        return container(space())
            .height(height)
            .width(Fill)
            .style(style::segment(None))
            .into();
    }

    Row::with_children(parts.into_iter().map(|(bytes, tone)| {
        let portion = ((bytes * 1000 / total) as u16).max(1);

        container(space())
            .width(Length::FillPortion(portion))
            .height(height)
            .style(style::segment(tone))
            .into()
    }))
    .spacing(2)
    .into()
}

pub fn dot<'a, Message: 'a>(tone: Option<Tone>) -> Element<'a, Message> {
    container(space())
        .width(8)
        .height(8)
        .style(style::segment(tone))
        .into()
}

pub fn table<'a, Message: 'a>(
    header: Element<'a, Message>,
    head: Element<'a, Message>,
    rows: impl IntoIterator<Item = Element<'a, Message>>,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut list = Column::new();
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            list = list.push(separator());
        }
        list = list.push(row);
    }

    container(column![
        header,
        container(head)
            .padding([8, 16])
            .width(Fill)
            .style(style::table_head),
        separator(),
        scrollable(list).height(Fill),
        separator(),
        container(footer).padding([12, 16]),
    ])
    .height(Fill)
    .style(style::card)
    .into()
}

pub fn check<'a, Message: 'a>(
    checked: bool,
    on_toggle: Option<impl Fn(bool) -> Message + 'a>,
) -> Element<'a, Message> {
    container(
        checkbox(checked)
            .size(16)
            .on_toggle_maybe(on_toggle)
            .style(style::check),
    )
    .width(CHECK_WIDTH)
    .into()
}

pub fn name<'a, Message: 'a>(name: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    container(
        text(truncate(name.into(), MAX_NAME_CHARS))
            .size(14)
            .wrapping(Wrapping::None),
    )
    .width(Fill)
    .clip(true)
    .into()
}

pub fn truncate(name: Cow<'_, str>, max_chars: usize) -> Cow<'_, str> {
    match name.char_indices().nth(max_chars) {
        Some((end, _)) => Cow::Owned(format!("{}…", &name[..end])),
        None => name,
    }
}

pub fn separator<'a, Message: 'a>() -> Element<'a, Message> {
    rule::horizontal(1).style(style::separator).into()
}

pub fn heading<'a, Message: 'a>(
    label: &'static str,
    width: impl Into<Length>,
    right: bool,
) -> Element<'a, Message> {
    let alignment = if right {
        Horizontal::Right
    } else {
        Horizontal::Left
    };

    container(text(label).size(12).font(style::MEDIUM).style(style::muted))
        .width(width)
        .align_x(alignment)
        .into()
}

pub fn sortable<'a, Message: Clone + 'a, Key: PartialEq>(
    label: &'static str,
    width: impl Into<Length>,
    right: bool,
    sort: Sort<Key>,
    key: Key,
    on_press: Message,
) -> Element<'a, Message> {
    let mut content = row![text(label).size(12).font(style::MEDIUM)]
        .spacing(4)
        .align_y(Center);

    if sort.key == key {
        let arrow = if sort.descending {
            Icon::ArrowDown
        } else {
            Icon::ArrowUp
        };
        content = content.push(self::icon(arrow).size(12));
    }

    let alignment = if right {
        Horizontal::Right
    } else {
        Horizontal::Left
    };

    container(
        button(content)
            .padding(0)
            .style(if sort.key == key {
                style::link_active
            } else {
                style::link
            })
            .on_press(on_press),
    )
    .width(width)
    .align_x(alignment)
    .into()
}

pub fn value<'a, Message: 'a>(
    value: impl IntoFragment<'a>,
    width: impl Into<Length>,
) -> Element<'a, Message> {
    container(text(value).size(14))
        .width(width)
        .align_x(Horizontal::Right)
        .into()
}

pub fn action<'a, Message: Clone + 'a>(
    label: &'static str,
    icon: Option<Icon>,
    style: fn(&Theme, button::Status) -> button::Style,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let mut content = row![].spacing(6).align_y(Center);

    if let Some(icon) = icon {
        content = content.push(self::icon(icon).size(14));
    }

    button(content.push(text(label).size(14).font(style::MEDIUM)))
        .padding([7, 12])
        .style(style)
        .on_press_maybe(on_press)
        .into()
}

pub fn icon_button<'a, Message: Clone + 'a>(
    icon: Icon,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(
        self::icon(icon)
            .size(16)
            .line_height(1.0)
            .width(Fill)
            .height(Fill)
            .center(),
    )
    .padding(0)
    .width(32)
    .height(32)
    .style(style::secondary)
    .on_press_maybe(on_press)
    .into()
}

pub fn selection(count: usize, bytes: u64) -> String {
    match count {
        0 => "Nothing selected".to_owned(),
        1 => format!("1 item selected  ·  {}", format::bytes(bytes)),
        count => format!("{count} items selected  ·  {}", format::bytes(bytes)),
    }
}
