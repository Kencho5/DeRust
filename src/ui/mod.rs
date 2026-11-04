pub mod donut;
pub mod explorer;
pub mod junk;
pub mod overview;
pub mod style;
pub mod uninstaller;

use std::borrow::Cow;
use std::path::Path;
use std::sync::LazyLock;

use iced::alignment::Horizontal;
use iced::widget::text::{IntoFragment, Wrapping};
use iced::widget::{
    Column, Row, Text, button, center, checkbox, column, container, image, progress_bar, row, rule,
    scrollable, space, text, tooltip,
};
use iced::{Center, Element, Fill, Length, Theme};
use lucide_icons::Icon;

use crate::format;
use crate::platform::{self, FileIcon};
use crate::scan::progress::Snapshot;
use style::Tone;

const CHECK_WIDTH: f32 = 28.0;

const MAX_NAME_CHARS: usize = 64;
const SIDEBAR_WIDTH: f32 = 240.0;
const LINE_ICON: f32 = 24.0;

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

pub fn progress<'a, Message: Clone + 'a>(
    label: &'static str,
    snapshot: Snapshot,
    stop: Message,
) -> Element<'a, Message> {
    let mut content = column![
        row![
            text(label)
                .size(14)
                .font(style::MEDIUM)
                .style(style::strong),
            space().width(Fill),
            text(format::bytes(snapshot.bytes))
                .size(14)
                .font(style::MEDIUM)
                .style(style::strong),
            action("Stop", Some(Icon::Square), style::secondary, Some(stop)),
        ]
        .spacing(16)
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
        (Icon::Timer, format::duration(snapshot.elapsed), 44.0),
        (
            Icon::Files,
            format!("{} files", format::count(snapshot.files)),
            96.0,
        ),
        (
            Icon::Gauge,
            format!("{} files/s", format::count(snapshot.files_per_second())),
            92.0,
        ),
        (
            Icon::Activity,
            format!("{}/s", format::bytes(snapshot.bytes_per_second())),
            64.0,
        ),
        (
            Icon::Cpu,
            format!("{} threads", rayon::current_num_threads()),
            60.0,
        ),
    ];

    Row::with_children(metrics.into_iter().map(|(icon, value, width)| {
        row![
            self::icon(icon).size(14).style(style::muted),
            text(value)
                .size(13)
                .style(style::muted)
                .wrapping(Wrapping::None)
                .width(width),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    }))
    .spacing(16)
    .into()
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

pub fn split<'a, Message: 'a>(
    sidebar: impl IntoIterator<Item = Element<'a, Message>>,
    content: Element<'a, Message>,
    status: String,
    action: Element<'a, Message>,
) -> Element<'a, Message> {
    let sidebar = scrollable(Column::with_children(sidebar).spacing(2).padding(12))
        .width(SIDEBAR_WIDTH)
        .height(Fill);

    let footer = row![
        text(status).size(14).style(style::muted),
        space().width(Fill),
        action,
    ]
    .align_y(Center);

    container(column![
        row![
            sidebar,
            rule::vertical(1).style(style::separator),
            container(content).width(Fill).height(Fill),
        ]
        .height(Fill),
        separator(),
        container(footer).padding([12, 20]),
    ])
    .height(Fill)
    .clip(true)
    .style(style::card)
    .into()
}

pub fn choice<'a, Message: Clone + 'a>(
    leading: Option<Element<'a, Message>>,
    label: impl IntoFragment<'a>,
    trailing: String,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let mut content = row![].spacing(10).align_y(Center);

    if let Some(leading) = leading {
        content = content.push(leading);
    }

    let content = content
        .push(
            container(text(label).size(14).wrapping(Wrapping::None))
                .width(Fill)
                .clip(true),
        )
        .push(text(trailing).size(13).style(style::muted));

    button(content)
        .padding([9, 12])
        .width(Fill)
        .style(style::choice(selected))
        .on_press(on_press)
        .into()
}

pub fn group<'a, Message: 'a>(label: &'static str) -> Element<'a, Message> {
    container(self::label(label)).padding([12, 12]).into()
}

pub fn line<'a, Message: Clone + 'a>(
    check: Option<Element<'a, Message>>,
    icon: Option<&FileIcon>,
    bytes: u64,
    name: impl Into<Cow<'a, str>>,
    reveal: Message,
) -> Element<'a, Message> {
    let check = check.unwrap_or_else(|| space().into());

    row![
        check,
        thumbnail(icon, LINE_ICON),
        container(
            text(truncate(name.into(), MAX_NAME_CHARS))
                .size(14)
                .wrapping(Wrapping::None),
        )
        .width(Fill)
        .clip(true),
        text(format::bytes(bytes)).size(14).style(style::muted),
        reveal_button(reveal),
    ]
    .spacing(12)
    .align_y(Center)
    .padding([8, 20])
    .into()
}

pub fn thumbnail<'a, Message: 'a>(icon: Option<&FileIcon>, size: f32) -> Element<'a, Message> {
    static FOLDER: LazyLock<Option<FileIcon>> = LazyLock::new(platform::folder_icon);

    match icon.or(FOLDER.as_ref()) {
        Some(icon) => image(icon.clone()).width(size).height(size).into(),
        None => space().width(size).height(size).into(),
    }
}

pub fn reveal_button<'a, Message: Clone + 'a>(on_press: Message) -> Element<'a, Message> {
    tooltip(
        button(self::icon(Icon::FolderSearch).size(14).line_height(1.0))
            .padding([2, 6])
            .style(style::link)
            .on_press(on_press),
        container(text("Reveal in Finder").size(12))
            .padding([4, 8])
            .style(style::tip),
        tooltip::Position::Left,
    )
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

pub fn label<'a, Message: 'a>(label: &'static str) -> Element<'a, Message> {
    text(label)
        .size(12)
        .font(style::MEDIUM)
        .style(style::muted)
        .into()
}

pub fn home_relative(path: &Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(home).ok().map(Path::to_owned)) {
        Some(relative) => format!("~/{}", relative.display()),
        None => path.display().to_string(),
    }
}

pub fn separator<'a, Message: 'a>() -> Element<'a, Message> {
    rule::horizontal(1).style(style::separator).into()
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
