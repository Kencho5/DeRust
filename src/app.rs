use std::thread;

use iced::futures::channel::oneshot;
use iced::widget::{Column, button, center, column, container, row, text};
use iced::{Center, Element, Fill, Task};

use crate::format;
use crate::junk::{self, Report};
use crate::section::Section;

#[derive(Debug, Default)]
pub struct App {
    section: Section,
    scan: Scan,
}

#[derive(Debug, Default)]
enum Scan {
    #[default]
    Idle,
    Running,
    Done(Report),
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Section),
    Scan,
    Scanned(Report),
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
                self.scan = Scan::Running;
                return Task::perform(background(junk::scan), Message::Scanned);
            }
            Message::Scanned(report) => self.scan = Scan::Done(report),
        }

        Task::none()
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
        let status = match &self.scan {
            Scan::Idle => text("Find caches, logs and build files you no longer need."),
            Scan::Running => text("Scanning..."),
            Scan::Done(report) => {
                let total = report.total();
                text(format!(
                    "Found {} in {} files",
                    format::bytes(total.bytes),
                    total.files
                ))
            }
        };

        let scan = button(text("Scan"))
            .padding([10, 24])
            .on_press_maybe((!matches!(self.scan, Scan::Running)).then_some(Message::Scan));

        center(
            column![text("System Junk").size(28), status, scan]
                .spacing(16)
                .align_x(Center),
        )
        .into()
    }
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
