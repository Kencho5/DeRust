use iced::Element;
use iced::widget::{center, text};

#[derive(Debug, Default)]
pub struct App;

#[derive(Debug, Clone)]
pub enum Message {}

impl App {
    pub const TITLE: &str = "Derust";

    pub fn new() -> Self {
        Self
    }

    pub fn update(&mut self, message: Message) {
        match message {}
    }

    pub fn view(&self) -> Element<'_, Message> {
        center(text("Derust").size(32)).into()
    }
}
