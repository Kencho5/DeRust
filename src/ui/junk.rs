use iced::widget::{row, space, text};
use iced::{Center, Element, Fill, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::Cleanup;
use crate::scan::junk::{self, Category, Report};
use crate::scan::progress::Snapshot;
use crate::task::{self, Update, background};
use crate::ui::style::{self, Tone};
use crate::ui::{self, Summary};

const TITLE: &str = "System Junk";
const SUBTITLE: &str = "Caches, logs and build files that apps recreate when needed.";

#[derive(Debug, Default)]
pub struct Junk {
    state: State,
    cleanup: Option<Cleanup>,
    performance: Option<Snapshot>,
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
    SelectAll(bool),
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
            Message::Scanning(Update::Finished(report, snapshot)) => {
                self.performance = Some(snapshot);

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
            Message::SelectAll(checked) => {
                if let State::Done { report, selected } = &mut self.state {
                    selected.clear();
                    if checked {
                        selected.extend(report.entries.iter().map(|entry| entry.category));
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

    pub fn summary(&self) -> Summary {
        match &self.state {
            State::Idle => Summary::Idle,
            State::Scanning(snapshot) => Summary::Scanning(*snapshot),
            State::Cleaning => Summary::Busy,
            State::Done { report, .. } => {
                let total = report.total();
                Summary::Found {
                    items: total.files,
                    bytes: total.bytes,
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rescan = matches!(self.state, State::Done { .. }).then(|| {
            ui::action(
                "Rescan",
                Some(Icon::RotateCw),
                style::secondary,
                Some(Message::Scan),
            )
        });

        let body = match &self.state {
            State::Idle => vec![ui::empty(
                Icon::Brush,
                "No scan yet",
                "Look through caches, logs and Xcode build data.",
                ui::action(
                    "Start scan",
                    Some(Icon::Play),
                    style::primary,
                    Some(Message::Scan),
                ),
            )],
            State::Scanning(snapshot) => vec![ui::progress("Scanning junk locations", *snapshot)],
            State::Cleaning => vec![ui::empty(
                Icon::Trash2,
                "Moving to Trash",
                "This only takes a moment.",
                space().into(),
            )],
            State::Done { report, selected } => results(report, selected, self.cleanup),
        };

        let actions = rescan.map(|rescan| ui::with_metrics(self.performance, rescan));

        ui::page(TITLE, SUBTITLE, actions, body)
    }
}

fn results<'a>(
    report: &'a Report,
    selected: &'a [Category],
    cleanup: Option<Cleanup>,
) -> Vec<Element<'a, Message>> {
    let total = report.total();

    let selected_bytes: u64 = report
        .entries
        .iter()
        .filter(|entry| selected.contains(&entry.category))
        .map(|entry| entry.usage.bytes)
        .sum();

    let caption = cleanup.map_or_else(
        || {
            format!(
                "{} files across {} locations, all safe to remove",
                format::count(total.files),
                report.entries.len()
            )
        },
        Cleanup::summary,
    );

    let header = ui::total(format::bytes(total.bytes), "of junk", caption)
        .push(space().height(6))
        .push(ui::segments(
            report
                .entries
                .iter()
                .map(|entry| (entry.usage.bytes, Some(tone(entry.category)))),
        ))
        .padding(16);

    let all = !report.entries.is_empty() && selected.len() == report.entries.len();

    let head = row![
        ui::check(all, Some(Message::SelectAll)),
        ui::heading("Name", Fill, false),
        ui::heading("Files", 120, true),
        ui::heading("Size", 120, true),
    ];

    let rows = report.entries.iter().map(|entry| {
        let category = entry.category;

        row![
            ui::check(
                selected.contains(&category),
                Some(move |checked| Message::Toggle(category, checked)),
            ),
            row![ui::dot(Some(tone(category))), ui::name(category.title())]
                .spacing(10)
                .align_y(Center)
                .width(Fill),
            ui::value(format::count(entry.usage.files), 120),
            ui::value(format::bytes(entry.usage.bytes), 120),
        ]
        .align_y(Center)
        .padding([10, 16])
        .into()
    });

    let footer = row![
        text(ui::selection(selected.len(), selected_bytes))
            .size(14)
            .style(style::muted),
        space().width(Fill),
        ui::action(
            "Clean up",
            Some(Icon::Trash2),
            style::primary,
            (!selected.is_empty()).then_some(Message::Clean),
        ),
    ]
    .align_y(Center);

    vec![ui::table(header.into(), head.into(), rows, footer.into())]
}

fn tone(category: Category) -> Tone {
    match category {
        Category::UserCaches => Tone::Orange,
        Category::UserLogs => Tone::Blue,
        Category::XcodeDerivedData => Tone::Green,
    }
}
