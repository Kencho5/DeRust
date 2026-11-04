use iced::{Element, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::Cleanup;
use crate::scan::junk::{self, Category, Report};
use crate::scan::progress::Snapshot;
use crate::task::{self, Update, background};
use crate::ui::{self, style};

const TITLE: &str = "System Junk";
const ICON: Icon = Icon::Brush;

#[derive(Debug, Default)]
pub struct Junk {
    state: State,
    cleanup: Option<Cleanup>,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Scanning(Snapshot),
    Cleaning,
    Done {
        report: Report,
        selected: Vec<Category>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanning(Update<Report>),
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
            Message::Scanning(Update::Progress(snapshot)) => {
                if let State::Scanning(current) = &mut self.state {
                    *current = snapshot;
                }
            }
            Message::Scanning(Update::Finished(report)) => {
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
        self.state = State::Scanning(Snapshot::default());
        Task::run(task::with_progress(junk::scan), Message::Scanning)
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Idle => ui::prompt(
                ICON,
                TITLE,
                "Find caches, logs and build files you no longer need.",
                Some(Message::Scan),
            ),
            State::Scanning(snapshot) => ui::scanning(ICON, TITLE, "Scanning...", *snapshot),
            State::Cleaning => ui::prompt(ICON, TITLE, "Moving files to Trash...", None),
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

        ui::item(
            selected.contains(&category),
            move |checked| Message::Toggle(category, checked),
            Icon::Folder,
            category.title(),
            format!("{} files", entry.usage.files),
            entry.usage.bytes,
        )
    });

    let footer = ui::footer(
        ui::selection(selected.len(), selected_bytes),
        [
            ui::action("Start Over", style::ghost, Some(Message::Scan)),
            ui::action(
                "Clean Up",
                style::remove,
                (!selected.is_empty()).then_some(Message::Clean),
            ),
        ],
    );

    ui::page(
        ICON,
        TITLE,
        format!(
            "Found {} in {} files",
            format::bytes(total.bytes),
            total.files
        ),
        cleanup.map(Cleanup::summary),
        rows,
        footer,
    )
}
