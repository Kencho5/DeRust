use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Center, Element, Fill};
use lucide_icons::Icon;

use crate::format;
use crate::scan::disk::Disk;
use crate::section::Section;
use crate::ui::style::{self, Tone};
use crate::ui::{self, Summary};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    ScanAll,
    StopAll,
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
    let summaries = [overview.junk, overview.explorer, overview.apps];
    let scanning = summaries
        .iter()
        .any(|summary| matches!(summary, Summary::Scanning(_)));
    let busy = summaries
        .iter()
        .any(|summary| matches!(summary, Summary::Busy));

    let scan_all = if scanning {
        ui::action(
            "Stop all",
            Some(Icon::Square),
            style::secondary,
            Some(Message::StopAll),
        )
    } else {
        ui::action(
            "Scan everything",
            Some(Icon::Play),
            style::primary,
            (!busy).then_some(Message::ScanAll),
        )
    };

    let disk: Element<'a, Message> = match overview.disk {
        Some(disk) => usage(disk, overview.junk, overview.apps),
        None => text("Disk information unavailable")
            .size(14)
            .style(style::muted)
            .into(),
    };

    let mut scans = Column::new();
    for (index, section) in Section::CLEANUP.into_iter().enumerate() {
        if index > 0 {
            scans = scans.push(ui::separator());
        }

        let summary = match section {
            Section::SystemJunk => overview.junk,
            Section::Explorer => overview.explorer,
            _ => overview.apps,
        };
        scans = scans.push(scan(section, summary));
    }

    ui::page(
        "Overview",
        "Here's what's taking up space on your Mac.",
        Some(scan_all),
        [
            container(disk)
                .padding(20)
                .width(Fill)
                .style(style::card)
                .into(),
            container(scans).width(Fill).style(style::card).into(),
        ],
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

    let parts = [
        ("Junk", junk, Some(Tone::Orange)),
        ("Apps", apps, Some(Tone::Blue)),
        ("Other", Some(other), Some(Tone::Neutral)),
        ("Available", Some(disk.available), None),
    ];

    let mut legend = row![].spacing(20);
    for (label, bytes, tone) in parts {
        let Some(bytes) = bytes else {
            continue;
        };

        legend = legend.push(
            row![
                ui::dot(tone),
                text(label).size(12).style(style::muted),
                text(format::bytes(bytes)).size(12),
            ]
            .spacing(6)
            .align_y(Center),
        );
    }

    column![
        row![
            text(format::bytes(disk.used()))
                .size(20)
                .font(style::MEDIUM)
                .style(style::strong),
            text(format!("of {} used", format::bytes(disk.total)))
                .size(14)
                .style(style::muted),
            space().width(Fill),
            text(format!(
                "{}% full",
                (disk.fraction() * 100.0).round() as u32
            ))
            .size(14)
            .style(style::muted),
        ]
        .spacing(8)
        .align_y(iced::Bottom),
        ui::bar(
            parts.map(|(_, bytes, tone)| (bytes.unwrap_or(0), tone)),
            6.0
        ),
        legend,
    ]
    .spacing(14)
    .into()
}

fn scan<'a>(section: Section, summary: Summary) -> Element<'a, Message> {
    let (status, found) = match summary {
        Summary::Idle => ("Not scanned".to_owned(), false),
        Summary::Scanning(snapshot) => (
            format!("Scanning  ·  {} files", format::count(snapshot.files)),
            false,
        ),
        Summary::Busy => ("Moving to Trash".to_owned(), false),
        Summary::Found { items, bytes } => (found_label(section, items, bytes), true),
    };

    let status_style: fn(&iced::Theme) -> text::Style =
        if found { style::strong } else { style::muted };

    let content = row![
        ui::icon(section.icon()).size(16).style(style::muted),
        text(section.title()).size(14).font(style::MEDIUM),
        space().width(Fill),
        text(status).size(14).style(status_style),
        ui::icon(Icon::ChevronRight).size(14).style(style::muted),
    ]
    .spacing(12)
    .align_y(Center);

    button(content)
        .padding([14, 20])
        .width(Fill)
        .style(style::row(false))
        .on_press(Message::Open(section))
        .into()
}

fn found_label(section: Section, items: u64, bytes: u64) -> String {
    match section {
        Section::Uninstaller => format!("{items} apps  ·  {}", format::bytes(bytes)),
        _ => format::bytes(bytes),
    }
}
