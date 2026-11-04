use std::path::PathBuf;

use iced::{Element, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::large_files::{self, LargeFile};
use crate::task::background;
use crate::ui::{self, style};

const TITLE: &str = "Large Files";
const ICON: Icon = Icon::FileStack;

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
                ICON,
                TITLE,
                "Find files over 100 MB in your home folder.",
                Some(Message::Scan),
            ),
            State::Scanning => ui::prompt(ICON, TITLE, "Scanning...", None),
            State::Cleaning => ui::prompt(ICON, TITLE, "Moving files to Trash...", None),
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

    let (selected_count, selected_bytes) = files
        .iter()
        .zip(selected)
        .filter(|&(_, &checked)| checked)
        .fold((0, 0), |(count, bytes), (file, _)| {
            (count + 1, bytes + file.bytes)
        });

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

            ui::item(
                checked,
                move |checked| Message::Toggle(index, checked),
                Icon::File,
                file.name(),
                location,
                file.bytes,
            )
        });

    let footer = ui::footer(
        ui::selection(selected_count, selected_bytes),
        [
            ui::action("Start Over", style::ghost, Some(Message::Scan)),
            ui::action(
                "Move to Trash",
                style::remove,
                (selected_count > 0).then_some(Message::Clean),
            ),
        ],
    );

    ui::page(
        ICON,
        TITLE,
        format!(
            "Found {} files totaling {}",
            files.len(),
            format::bytes(total)
        ),
        cleanup.map(Cleanup::summary),
        rows,
        footer,
    )
}
