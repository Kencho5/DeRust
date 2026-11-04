use iced::widget::{button, column, container, row, space, text};
use iced::{Center, Element, Fill};
use lucide_icons::Icon;

use crate::format;
use crate::scan::disk::Disk;
use crate::section::Section;
use crate::ui::style::{self, Tone};
use crate::ui::{self, Summary};

const STATUS_WIDTH: f32 = 200.0;
const FOUND_WIDTH: f32 = 140.0;
const ACTION_WIDTH: f32 = 96.0;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    ScanAll,
    Scan(Section),
    Open(Section),
}

#[derive(Debug, Clone, Copy)]
pub struct Overview {
    pub disk: Option<Disk>,
    pub junk: Summary,
    pub explorer: Summary,
    pub apps: Summary,
}

pub fn view<'a>(overview: Overview) -> Element<'a, Message> {
    let busy = [overview.junk, overview.explorer, overview.apps]
        .iter()
        .any(|summary| matches!(summary, Summary::Scanning(_) | Summary::Busy));

    let scan_all = ui::action(
        "Scan everything",
        Some(Icon::Play),
        style::primary,
        (!busy).then_some(Message::ScanAll),
    );

    let header: Element<'a, Message> = match overview.disk {
        Some(disk) => usage(disk, overview.junk, overview.apps),
        None => container(
            text("Disk information unavailable")
                .size(14)
                .style(style::muted),
        )
        .padding(20)
        .into(),
    };

    let head = row![
        ui::heading("Scan", Fill, false),
        ui::heading("Status", STATUS_WIDTH, false),
        ui::heading("Found", FOUND_WIDTH, true),
        space().width(ACTION_WIDTH),
    ];

    let rows = Section::CLEANUP.map(|section| {
        let summary = match section {
            Section::SystemJunk => overview.junk,
            Section::Explorer => overview.explorer,
            _ => overview.apps,
        };
        scan(section, summary)
    });

    let footer = text("Nothing is removed until you review it and confirm.")
        .size(14)
        .style(style::muted);

    ui::page(
        "Overview",
        "Here's what's taking up space on your Mac.",
        Some(scan_all),
        [ui::table(header, head.into(), rows, footer.into())],
    )
}

fn usage<'a>(disk: Disk, junk: Summary, apps: Summary) -> Element<'a, Message> {
    let found = |summary: Summary| match summary {
        Summary::Found { bytes, .. } => Some(bytes),
        _ => None,
    };

    let junk = found(junk);
    let apps = found(apps);
    let other = disk
        .used()
        .saturating_sub(junk.unwrap_or(0) + apps.unwrap_or(0));

    let share = |bytes: u64| {
        if disk.total == 0 {
            0
        } else {
            bytes * 100 / disk.total
        }
    };

    let mut legend = row![].spacing(12);
    for (label, bytes, tone) in [
        ("Junk", junk, Some(Tone::Orange)),
        ("Apps", apps, Some(Tone::Blue)),
        ("Other", Some(other), Some(Tone::Neutral)),
        ("Available", Some(disk.available), None),
    ] {
        let (value, caption) = match bytes {
            Some(bytes) => (format::bytes(bytes), format!("{}% of disk", share(bytes))),
            None => ("—".to_owned(), "Not scanned".to_owned()),
        };

        legend = legend.push(
            column![
                row![
                    ui::dot(tone),
                    text(label).size(12).font(style::MEDIUM).style(style::muted),
                ]
                .spacing(8)
                .align_y(Center),
                text(value)
                    .size(18)
                    .font(style::SEMIBOLD)
                    .style(style::strong),
                text(caption).size(12).style(style::muted),
            ]
            .spacing(4)
            .width(Fill),
        );
    }

    let header = row![
        column![
            text("Macintosh HD")
                .size(12)
                .font(style::MEDIUM)
                .style(style::muted),
            row![
                text(format::bytes(disk.used()))
                    .size(36)
                    .font(style::SEMIBOLD)
                    .style(style::strong),
                text(format!("of {} used", format::bytes(disk.total)))
                    .size(14)
                    .style(style::muted),
            ]
            .spacing(10)
            .align_y(Center),
        ]
        .spacing(2),
        space().width(Fill),
        column![
            text(format!("{}%", (disk.fraction() * 100.0).round() as u32))
                .size(36)
                .font(style::SEMIBOLD)
                .style(style::accent),
            text("full").size(12).style(style::muted),
        ]
        .spacing(2)
        .align_x(iced::Right),
    ]
    .align_y(iced::Bottom);

    column![
        header,
        ui::bar(
            [
                (junk.unwrap_or(0), Some(Tone::Orange)),
                (apps.unwrap_or(0), Some(Tone::Blue)),
                (other, Some(Tone::Neutral)),
                (disk.available, None),
            ],
            12.0,
        ),
        space().height(2),
        legend,
    ]
    .spacing(18)
    .padding(20)
    .into()
}

fn scan<'a>(section: Section, summary: Summary) -> Element<'a, Message> {
    let description = match section {
        Section::SystemJunk => "Caches, logs and Xcode build data",
        Section::Explorer => "Every folder on your disk, largest first",
        _ => "Installed apps and the files they leave behind",
    };

    let (status, found) = match summary {
        Summary::Idle => ("Not scanned".to_owned(), "—".to_owned()),
        Summary::Scanning(snapshot) => (
            format!("Scanning  ·  {} files", format::count(snapshot.files)),
            format::bytes(snapshot.bytes),
        ),
        Summary::Busy => ("Moving to Trash".to_owned(), "—".to_owned()),
        Summary::Found { items, bytes } => ("Ready".to_owned(), found(section, items, bytes)),
    };

    let status_style: fn(&iced::Theme) -> text::Style = match summary {
        Summary::Found { .. } => style::strong,
        _ => style::muted,
    };

    let action = match summary {
        Summary::Idle => ui::action("Scan", None, style::primary, Some(Message::Scan(section))),
        _ => ui::action("Open", None, style::secondary, Some(Message::Open(section))),
    };

    let content = row![
        row![
            ui::icon(section.icon()).size(16).style(style::muted),
            column![
                text(section.title())
                    .size(14)
                    .font(style::MEDIUM)
                    .style(style::strong),
                text(description).size(12).style(style::muted),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(Center)
        .width(Fill),
        container(text(status).size(14).style(status_style)).width(STATUS_WIDTH),
        ui::value(found, FOUND_WIDTH),
        container(action)
            .width(ACTION_WIDTH)
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .align_y(Center);

    button(content)
        .padding([10, 16])
        .width(Fill)
        .style(style::row(false))
        .on_press(Message::Open(section))
        .into()
}

fn found(section: Section, items: u64, bytes: u64) -> String {
    match section {
        Section::Uninstaller => format!("{items} apps  ·  {}", format::bytes(bytes)),
        _ => format::bytes(bytes),
    }
}
