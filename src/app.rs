use iced::widget::{Row, button, column, container, mouse_area, row, space, text};
use iced::{Center, Element, Fill, Padding, Task, Theme, window};

use crate::platform;
use crate::scan::disk::Disk;
use crate::section::Section;
use crate::ui::explorer::{self, Explorer};
use crate::ui::junk::{self, Junk};
use crate::ui::overview::{self, Overview};
use crate::ui::uninstaller::{self, Uninstaller};
use crate::ui::{self, style};

const TRAFFIC_LIGHTS_END: f32 = 72.0;
const HEADER_GAP: f32 = 28.0;
const HEADER_HEIGHT: f32 = 52.0;

#[derive(Debug)]
pub struct App {
    section: Section,
    disk: Option<Disk>,
    junk: Junk,
    explorer: Explorer,
    uninstaller: Uninstaller,
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Section),
    Drag,
    Overview(overview::Message),
    Junk(junk::Message),
    Explorer(explorer::Message),
    Uninstaller(uninstaller::Message),
}

impl App {
    pub const TITLE: &str = "Derust";

    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            section: Section::default(),
            disk: Disk::root(),
            junk: Junk::default(),
            explorer: Explorer::default(),
            uninstaller: Uninstaller::default(),
        };

        let titlebar = window::oldest()
            .and_then(|id| window::run(id, platform::unify_titlebar))
            .discard();

        (app, titlebar)
    }

    pub fn theme(&self) -> Theme {
        style::theme()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Select(section) | Message::Overview(overview::Message::Open(section)) => {
                self.section = section;
                Task::none()
            }
            Message::Drag => window::latest().and_then(window::drag),
            Message::Overview(overview::Message::ScanAll) => {
                Task::batch(Section::CLEANUP.map(|section| self.scan(section)))
            }
            Message::Overview(overview::Message::StopAll) => Task::batch([
                self.update(Message::Junk(junk::Message::Stop)),
                self.update(Message::Explorer(explorer::Message::Stop)),
                self.update(Message::Uninstaller(uninstaller::Message::Stop)),
            ]),
            Message::Junk(message) => {
                self.disk = Disk::root();
                self.junk.update(message).map(Message::Junk)
            }
            Message::Explorer(message) => {
                self.disk = Disk::root();
                self.explorer.update(message).map(Message::Explorer)
            }
            Message::Uninstaller(message) => {
                self.disk = Disk::root();
                self.uninstaller.update(message).map(Message::Uninstaller)
            }
        }
    }

    fn scan(&mut self, section: Section) -> Task<Message> {
        match section {
            Section::SystemJunk => self.update(Message::Junk(junk::Message::Scan)),
            Section::Explorer => self.update(Message::Explorer(explorer::Message::Scan)),
            Section::Uninstaller => self.update(Message::Uninstaller(uninstaller::Message::Scan)),
            Section::Overview => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.section {
            Section::Overview => overview::view(Overview {
                disk: self.disk,
                junk: self.junk.summary(),
                explorer: self.explorer.summary(),
                apps: self.uninstaller.summary(),
            })
            .map(Message::Overview),
            Section::SystemJunk => self.junk.view().map(Message::Junk),
            Section::Explorer => self.explorer.view().map(Message::Explorer),
            Section::Uninstaller => self.uninstaller.view().map(Message::Uninstaller),
        };

        container(column![
            self.header(),
            container(content).width(Fill).height(Fill).padding(24),
        ])
        .width(Fill)
        .height(Fill)
        .style(style::window)
        .into()
    }

    fn header(&self) -> Element<'_, Message> {
        let brand = row![
            text("de")
                .size(17)
                .font(style::SEMIBOLD)
                .style(style::accent),
            text("rust")
                .size(17)
                .font(style::SEMIBOLD)
                .style(style::strong),
        ];

        let tabs = Row::with_children(Section::ALL.map(|section| {
            button(
                row![
                    ui::icon(section.icon()).size(14),
                    text(section.title()).size(14).font(style::MEDIUM),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .padding([6, 10])
            .style(style::nav(section == self.section))
            .on_press(Message::Select(section))
            .into()
        }))
        .spacing(4);

        let bar = container(
            row![brand, space().width(HEADER_GAP), tabs, space().width(Fill)].align_y(Center),
        )
        .padding(Padding {
            left: TRAFFIC_LIGHTS_END + HEADER_GAP,
            right: 24.0,
            ..Padding::ZERO
        })
        .center_y(HEADER_HEIGHT);

        column![mouse_area(bar).on_press(Message::Drag), ui::separator()].into()
    }
}
