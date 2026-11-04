use iced::widget::{Column, button, center, container, row, text};
use iced::{Element, Fill, Task};

use crate::section::Section;
use crate::ui::junk::{self, Junk};
use crate::ui::large_files::{self, LargeFiles};
use crate::ui::uninstaller::{self, Uninstaller};

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
            Section::SystemJunk => self.junk.view().map(Message::Junk),
            Section::LargeFiles => self.large_files.view().map(Message::LargeFiles),
            Section::Uninstaller => self.uninstaller.view().map(Message::Uninstaller),
            section => center(text(section.title()).size(28)).into(),
        }
    }
}
