use std::path::PathBuf;

use iced::widget::{Column, button, checkbox, column, row, scrollable, space, text};
use iced::{Center, Element, Fill, Task};

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::large_files::{self, LargeFile};
use crate::task::background;
use crate::ui;

const TITLE: &str = "Large Files";

#[derive(Debug, Default)]
pub struct LargeFiles {
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
        files: Vec<LargeFile>,
        selected: Vec<bool>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanned(Vec<LargeFile>),
    Toggle(usize, bool),
    Clean,
    Cleaned(Cleanup),
}

impl LargeFiles {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scan => {
                self.cleanup = None;
                return self.start_scan();
            }
            Message::Scanned(files) => {
                let selected = vec![false; files.len()];
                self.state = State::Done { files, selected };
            }
            Message::Toggle(index, checked) => {
                if let State::Done { selected, .. } = &mut self.state
                    && let Some(slot) = selected.get_mut(index)
                {
                    *slot = checked;
                }
            }
            Message::Clean => {
                if let State::Done { files, selected } = &self.state {
                    let paths: Vec<PathBuf> = files
                        .iter()
                        .zip(selected)
                        .filter(|&(_, &checked)| checked)
                        .map(|(file, _)| file.path.clone())
                        .collect();

                    self.state = State::Cleaning;

                    return Task::perform(
                        background(move || recycle::move_to_trash(&paths)),
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
        Task::perform(background(large_files::scan), Message::Scanned)
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Idle => ui::prompt(
                TITLE,
                "Find files over 100 MB in your home folder.",
                Some(Message::Scan),
            ),
            State::Scanning => ui::prompt(TITLE, "Scanning...", None),
            State::Cleaning => ui::prompt(TITLE, "Moving files to Trash...", None),
            State::Done { files, selected } => results(files, selected, self.cleanup),
        }
    }
}

fn results<'a>(
    files: &'a [LargeFile],
    selected: &'a [bool],
    cleanup: Option<Cleanup>,
) -> Element<'a, Message> {
    let total: u64 = files.iter().map(|file| file.bytes).sum();

    let selected_bytes: u64 = files
        .iter()
        .zip(selected)
        .filter(|&(_, &checked)| checked)
        .map(|(file, _)| file.bytes)
        .sum();

    let rows = files
        .iter()
        .zip(selected)
        .enumerate()
        .map(|(index, (file, &checked))| {
            let location = file
                .path
                .parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default();

            row![
                checkbox(checked).on_toggle(move |checked| Message::Toggle(index, checked)),
                column![text(file.name()), text(location).size(12)].spacing(2),
                space().width(Fill),
                text(format::bytes(file.bytes)),
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
        button(text("Move to Trash"))
            .padding([10, 24])
            .on_press_maybe((selected_bytes > 0).then_some(Message::Clean)),
    ]
    .spacing(12)
    .align_y(Center);

    let mut header = column![
        text(TITLE).size(28),
        text(format!(
            "Found {} files totaling {}",
            files.len(),
            format::bytes(total)
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
