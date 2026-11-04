use std::thread;

use iced::futures::channel::oneshot;
use iced::widget::{
    Column, button, center, checkbox, column, container, row, scrollable, space, text,
};
use iced::{Center, Element, Fill, Task};

use crate::format;
use crate::junk::{self, Category, Cleanup, Report};
use crate::section::Section;

#[derive(Debug, Default)]
pub struct App {
    section: Section,
    scan: Scan,
    cleanup: Option<Cleanup>,
}

#[derive(Debug, Default)]
enum Scan {
    #[default]
    Idle,
    Running,
    Cleaning,
    Done {
        report: Report,
        selected: Vec<Category>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Section),
    Scan,
    Scanned(Report),
    Toggle(Category, bool),
    Clean,
    Cleaned(Cleanup),
}

impl App {
    pub const TITLE: &str = "Derust";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(section) => self.section = section,
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

                self.scan = Scan::Done { report, selected };
            }
            Message::Toggle(category, checked) => {
                if let Scan::Done { selected, .. } = &mut self.scan {
                    if checked {
                        selected.push(category);
                    } else {
                        selected.retain(|&c| c != category);
                    }
                }
            }
            Message::Clean => {
                if let Scan::Done { selected, .. } = &self.scan {
                    let categories = selected.clone();
                    self.scan = Scan::Cleaning;

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
        self.scan = Scan::Running;
        Task::perform(background(junk::scan), Message::Scanned)
    }

    pub fn view(&self) -> Element<'_, Message> {
        row![self.sidebar(), self.content()].into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let items = Section::ALL.into_iter().map(|section| {
            let style = if section == self.section {
                button::primary
            } else {
                button::text
            };

            button(text(section.title()))
                .width(Fill)
                .padding([8, 12])
                .style(style)
                .on_press(Message::Select(section))
                .into()
        });

        container(Column::with_children(items).spacing(4))
            .width(220)
            .height(Fill)
            .padding(12)
            .style(container::bordered_box)
            .into()
    }

    fn content(&self) -> Element<'_, Message> {
        match self.section {
            Section::SystemJunk => self.system_junk(),
            section => center(text(section.title()).size(28)).into(),
        }
    }

    fn system_junk(&self) -> Element<'_, Message> {
        match &self.scan {
            Scan::Idle => scan_prompt(
                "Find caches, logs and build files you no longer need.",
                Some(Message::Scan),
            ),
            Scan::Running => scan_prompt("Scanning...", None),
            Scan::Cleaning => scan_prompt("Moving files to Trash...", None),
            Scan::Done { report, selected } => results(report, selected, self.cleanup),
        }
    }
}

fn scan_prompt<'a>(status: &'static str, on_scan: Option<Message>) -> Element<'a, Message> {
    let scan = button(text("Scan"))
        .padding([10, 24])
        .on_press_maybe(on_scan);

    center(
        column![text("System Junk").size(28), text(status), scan]
            .spacing(16)
            .align_x(Center),
    )
    .into()
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
        text("System Junk").size(28),
        text(format!(
            "Found {} in {} files",
            format::bytes(total.bytes),
            total.files
        )),
    ]
    .spacing(8);

    if let Some(cleanup) = cleanup {
        let mut notice = format!("Moved {} items to Trash", cleanup.trashed);

        if cleanup.failed > 0 {
            notice.push_str(&format!(", {} could not be moved", cleanup.failed));
        }

        header = header.push(text(notice));
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

fn background<T, F>(job: F) -> impl Future<Output = T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = oneshot::channel();

    thread::spawn(move || {
        let _ = sender.send(job());
    });

    async move { receiver.await.expect("background job panicked") }
}
