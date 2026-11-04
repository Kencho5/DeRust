use iced::widget::{Column, button, checkbox, column, row, scrollable, space, text};
use iced::{Center, Element, Fill, Task};

use crate::format;
use crate::recycle::Cleanup;
use crate::scan::junk::{self, Category, Report};
use crate::task::background;
use crate::ui;

const TITLE: &str = "System Junk";

#[derive(Debug, Default)]
pub struct Junk {
    state: State,
    cleanup: Option<Cleanup>,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Scanning,
    Cleaning,
    Done {
        report: Report,
        selected: Vec<Category>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanned(Report),
    Toggle(Category, bool),
    Clean,
    Cleaned(Cleanup),
}

impl Junk {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scan => {
                self.cleanup = None;
                return self.start_scan();
            }
            Message::Scanned(report) => {
                let selected = report
                    .entries
                    .iter()
                    .filter(|entry| entry.usage.bytes > 0)
                    .map(|entry| entry.category)
                    .collect();

                self.state = State::Done { report, selected };
            }
            Message::Toggle(category, checked) => {
                if let State::Done { selected, .. } = &mut self.state {
                    if checked {
                        selected.push(category);
                    } else {
                        selected.retain(|&c| c != category);
                    }
                }
            }
            Message::Clean => {
                if let State::Done { selected, .. } = &self.state {
                    let categories = selected.clone();
                    self.state = State::Cleaning;

                    return Task::perform(
                        background(move || junk::clean(&categories)),
                        Message::Cleaned,
                    );
                }
            }
            Message::Cleaned(cleanup) => {
                self.cleanup = Some(cleanup);
                return self.start_scan();
            }
        }

        Task::none()
    }

    fn start_scan(&mut self) -> Task<Message> {
        self.state = State::Scanning;
        Task::perform(background(junk::scan), Message::Scanned)
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Idle => ui::prompt(
                TITLE,
                "Find caches, logs and build files you no longer need.",
                Some(Message::Scan),
            ),
            State::Scanning => ui::prompt(TITLE, "Scanning...", None),
            State::Cleaning => ui::prompt(TITLE, "Moving files to Trash...", None),
            State::Done { report, selected } => results(report, selected, self.cleanup),
        }
    }
}

fn results<'a>(
    report: &'a Report,
    selected: &'a [Category],
    cleanup: Option<Cleanup>,
) -> Element<'a, Message> {
    let total = report.total();

    let selected_bytes: u64 = report
        .entries
        .iter()
        .filter(|entry| selected.contains(&entry.category))
        .map(|entry| entry.usage.bytes)
        .sum();

    let rows = report.entries.iter().map(|entry| {
        let category = entry.category;

        row![
            checkbox(selected.contains(&category))
                .label(category.title())
                .on_toggle(move |checked| Message::Toggle(category, checked)),
            space().width(Fill),
            text(format::bytes(entry.usage.bytes)),
        ]
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
        button(text("Clean"))
            .padding([10, 24])
            .on_press_maybe((!selected.is_empty()).then_some(Message::Clean)),
    ]
    .spacing(12)
    .align_y(Center);

    let mut header = column![
        text(TITLE).size(28),
        text(format!(
            "Found {} in {} files",
            format::bytes(total.bytes),
            total.files
        )),
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
