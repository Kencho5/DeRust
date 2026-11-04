use iced::widget::{Column, button, center, column, container, row, text, tooltip};
use iced::{Center, Element, Fill, Padding, Task, Theme};

use crate::section::Section;
use crate::ui::junk::{self, Junk};
use crate::ui::large_files::{self, LargeFiles};
use crate::ui::uninstaller::{self, Uninstaller};
use crate::ui::{self, style};

#[derive(Debug, Default)]
pub struct App {
    section: Section,
    junk: Junk,
    large_files: LargeFiles,
    uninstaller: Uninstaller,
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Section),
    Junk(junk::Message),
    LargeFiles(large_files::Message),
    Uninstaller(uninstaller::Message),
}

impl App {
    pub const TITLE: &str = "Derust";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn theme(&self) -> Theme {
        style::theme()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(section) => {
                self.section = section;
                Task::none()
            }
            Message::Junk(message) => self.junk.update(message).map(Message::Junk),
            Message::LargeFiles(message) => {
                self.large_files.update(message).map(Message::LargeFiles)
            }
            Message::Uninstaller(message) => {
                self.uninstaller.update(message).map(Message::Uninstaller)
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(row![self.sidebar(), self.main()])
            .width(Fill)
            .height(Fill)
            .style(style::window)
            .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let items = Section::ALL.into_iter().map(|section| {
            let tile = button(center(ui::icon(section.icon()).size(24)))
                .width(56)
                .height(56)
                .style(style::nav(section == self.section))
                .on_press(Message::Select(section));

            let label = container(text(section.title()).size(13))
                .padding([6, 10])
                .style(style::tooltip);

            tooltip(tile, label, tooltip::Position::Right).into()
        });

        container(Column::with_children(items).spacing(12))
            .width(88)
            .height(Fill)
            .align_x(Center)
            .padding(Padding {
                top: 56.0,
                bottom: 16.0,
                ..Padding::ZERO
            })
            .style(style::sidebar)
            .into()
    }

    fn main(&self) -> Element<'_, Message> {
        let title = container(text(self.section.title()).size(15).style(style::subtle))
            .width(Fill)
            .center_x(Fill)
            .center_y(44);

        let content = match self.section {
            Section::SystemJunk => self.junk.view().map(Message::Junk),
            Section::LargeFiles => self.large_files.view().map(Message::LargeFiles),
            Section::Uninstaller => self.uninstaller.view().map(Message::Uninstaller),
            section => ui::placeholder(section.icon(), section.title()),
        };

        column![
            title,
            container(content).padding(Padding {
                top: 8.0,
                right: 24.0,
                bottom: 24.0,
                left: 24.0,
            })
        ]
        .into()
    }
}
