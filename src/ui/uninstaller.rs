use std::path::PathBuf;

use iced::widget::{Column, button, checkbox, column, row, scrollable, space, text};
use iced::{Center, Element, Fill, Task};

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::apps::{self, InstalledApp};
use crate::task::background;
use crate::ui;

const TITLE: &str = "Uninstaller";

#[derive(Debug, Default)]
pub struct Uninstaller {
    state: State,
    cleanup: Option<Cleanup>,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Scanning,
    Removing,
    Done {
        apps: Vec<InstalledApp>,
        selected: Vec<bool>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanned(Vec<InstalledApp>),
    Toggle(usize, bool),
    Uninstall,
    Uninstalled(Cleanup),
}

impl Uninstaller {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scan => {
                self.cleanup = None;
                return self.start_scan();
            }
            Message::Scanned(apps) => {
                let selected = vec![false; apps.len()];
                self.state = State::Done { apps, selected };
            }
            Message::Toggle(index, checked) => {
                if let State::Done { selected, .. } = &mut self.state
                    && let Some(slot) = selected.get_mut(index)
                {
                    *slot = checked;
                }
            }
            Message::Uninstall => {
                if let State::Done { apps, selected } = &self.state {
                    let paths: Vec<PathBuf> = apps
                        .iter()
                        .zip(selected)
                        .filter(|&(_, &checked)| checked)
                        .flat_map(|(app, _)| app.paths().cloned())
                        .collect();

                    self.state = State::Removing;

                    return Task::perform(
                        background(move || recycle::move_to_trash(&paths)),
                        Message::Uninstalled,
                    );
                }
            }
            Message::Uninstalled(cleanup) => {
                self.cleanup = Some(cleanup);
                return self.start_scan();
            }
        }

        Task::none()
    }

    fn start_scan(&mut self) -> Task<Message> {
        self.state = State::Scanning;
        Task::perform(background(apps::scan), Message::Scanned)
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Idle => ui::prompt(
                TITLE,
                "Remove apps along with the files they leave behind.",
                Some(Message::Scan),
            ),
            State::Scanning => ui::prompt(TITLE, "Looking for apps...", None),
            State::Removing => ui::prompt(TITLE, "Moving apps to Trash...", None),
            State::Done { apps, selected } => results(apps, selected, self.cleanup),
        }
    }
}

fn results<'a>(
    apps: &'a [InstalledApp],
    selected: &'a [bool],
    cleanup: Option<Cleanup>,
) -> Element<'a, Message> {
    let selected_bytes: u64 = apps
        .iter()
        .zip(selected)
        .filter(|&(_, &checked)| checked)
        .map(|(app, _)| app.total_bytes())
        .sum();

    let rows = apps
        .iter()
        .zip(selected)
        .enumerate()
        .map(|(index, (app, &checked))| {
            let details = match app.leftovers.len() {
                0 => "No leftovers".to_owned(),
                1 => format!("1 leftover, {}", format::bytes(app.leftover_bytes)),
                count => format!("{count} leftovers, {}", format::bytes(app.leftover_bytes)),
            };

            row![
                checkbox(checked).on_toggle(move |checked| Message::Toggle(index, checked)),
                column![text(&app.name), text(details).size(12)].spacing(2),
                space().width(Fill),
                text(format::bytes(app.total_bytes())),
            ]
            .spacing(12)
            .align_y(Center)
            .into()
        });

    let footer = row![
        text(format!("{} selected", format::bytes(selected_bytes))),
        space().width(Fill),
        button(text("Rescan"))
            .padding([10, 24])
            .style(button::secondary)
            .on_press(Message::Scan),
        button(text("Uninstall"))
            .padding([10, 24])
            .style(button::danger)
            .on_press_maybe(selected.contains(&true).then_some(Message::Uninstall)),
    ]
    .spacing(12)
    .align_y(Center);

    let mut header = column![
        text(TITLE).size(28),
        text(format!("{} apps installed", apps.len())),
    ]
    .spacing(8);

    if let Some(cleanup) = cleanup {
        header = header.push(text(cleanup.summary()));
    }

    column![
        header,
        scrollable(Column::with_children(rows).spacing(12)).height(Fill),
        footer,
    ]
    .spacing(16)
    .padding(24)
    .into()
}
