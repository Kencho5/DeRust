use iced::widget::{Column, column, container, row, space, text};
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

    let mut scans = Column::new().push(ui::separator());
    for section in Section::CLEANUP {
        let summary = match section {
            Section::SystemJunk => overview.junk,
            Section::Explorer => overview.explorer,
            _ => overview.apps,
        };
        scans = scans.push(scan(section, summary)).push(ui::separator());
    }

    let note = text("Nothing is removed until you review it and confirm.")
        .size(13)
        .style(style::muted);

    let mut body = Vec::new();
    if let Some(disk) = overview.disk {
        body.push(usage(disk, overview.junk, overview.apps));
        body.push(space().height(8).into());
    }
    body.push(scans.into());
    body.push(note.into());

    ui::page(
        "Overview",
        "Here's what's taking up space on your Mac.",
        Some(scan_all),
        body,
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

    let mut legend = row![].spacing(20).align_y(Center);
    for (label, bytes, tone) in [
        ("Junk", junk, Some(Tone::Orange)),
        ("Apps", apps, Some(Tone::Blue)),
        ("Other", Some(other), Some(Tone::Neutral)),
        ("Available", Some(disk.available), None),
    ] {
        if let Some(bytes) = bytes {
            legend = legend.push(
                row![
                    ui::dot(tone),
                    text(format!("{label}  {}", format::bytes(bytes)))
                        .size(13)
                        .style(style::muted),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
    }

    column![
        row![
            text(format::bytes(disk.used()))
                .size(32)
                .font(style::SEMIBOLD)
                .style(style::strong),
            text(format!("of {} used", format::bytes(disk.total)))
                .size(14)
                .style(style::muted),
        ]
        .spacing(10)
        .align_y(Center),
        ui::segments([
            (junk.unwrap_or(0), Some(Tone::Orange)),
            (apps.unwrap_or(0), Some(Tone::Blue)),
            (other, Some(Tone::Neutral)),
            (disk.available, None),
        ]),
        legend,
    ]
    .spacing(12)
    .into()
}

fn scan<'a>(section: Section, summary: Summary) -> Element<'a, Message> {
    let description = match section {
        Section::SystemJunk => "Caches, logs and Xcode build data",
        Section::Explorer => "Every folder on your disk, largest first",
        _ => "Installed apps and the files they leave behind",
    };

    let status = match summary {
        Summary::Idle => "Not scanned".to_owned(),
        Summary::Scanning(snapshot) => {
            format!("Scanning  ·  {} files", format::count(snapshot.files))
        }
        Summary::Busy => "Moving to Trash".to_owned(),
        Summary::Found { items, bytes } => match section {
            Section::SystemJunk => format!("{} to clean", format::bytes(bytes)),
            Section::Explorer => format!("{} mapped", format::bytes(bytes)),
            _ => format!("{items} apps  ·  {}", format::bytes(bytes)),
        },
    };

    let status = if matches!(summary, Summary::Found { .. }) {
        text(status).size(14).style(style::strong)
    } else {
        text(status).size(14).style(style::muted)
    };

    let action = match summary {
        Summary::Idle => ui::action("Scan", None, style::primary, Some(Message::Scan(section))),
        _ => ui::action("Open", None, style::secondary, Some(Message::Open(section))),
    };

    row![
        container(ui::icon(section.icon()).size(18).style(style::muted)).width(32),
        column![
            text(section.title())
                .size(15)
                .font(style::MEDIUM)
                .style(style::strong),
            text(description).size(13).style(style::muted),
        ]
        .spacing(2),
        space().width(Fill),
        status,
        space().width(16),
        action,
    ]
    .align_y(Center)
    .padding([14, 0])
    .into()
}
