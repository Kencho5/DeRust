use iced::widget::{Column, button, center, container, row, text};
use iced::{Element, Fill};

use crate::section::Section;

#[derive(Debug, Default)]
pub struct App {
    section: Section,
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Section),
}

impl App {
    pub const TITLE: &str = "Derust";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Select(section) => self.section = section,
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
        center(text(self.section.title()).size(28)).into()
    }
}
