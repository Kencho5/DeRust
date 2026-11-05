use std::path::PathBuf;

use iced::widget::{Column, column, container, row, scrollable, space, text};
use iced::{Center, Element, Fill, Task};
use lucide_icons::Icon;

use crate::recycle::Cleanup;
use crate::scan::junk::{self, Category, Entry, Report};
use crate::scan::progress::Snapshot;
use crate::task::{self, Running, Update, background};
use crate::ui::style;
use crate::ui::{self, Summary};
use crate::{format, platform};

const TITLE: &str = "System Junk";
const SUBTITLE: &str = "Caches, logs and build files that apps recreate when needed.";

#[derive(Debug, Default)]
pub struct Junk {
    state: State,
    focus: usize,
    cleanup: Option<Cleanup>,
    performance: Option<Snapshot>,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Scanning(Running),
    Cleaning,
    Done {
        report: Report,
        selected: Vec<Vec<bool>>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanning(Update<Report>),
    Stop,
    Toggle(usize, bool),
    ToggleItem(usize, usize, bool),
    SelectAll(bool),
    Focus(usize),
    Reveal(PathBuf),
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
                if let State::Scanning(running) = &mut self.state {
                    running.snapshot = snapshot;
                }
            }
            Message::Stop => {
                if matches!(self.state, State::Scanning(_)) {
                    self.state = State::Idle;
                }
            }
            Message::Scanning(Update::Finished(report, snapshot)) => {
                self.performance = Some(snapshot);

                let selected = report
                    .entries
                    .iter()
                    .map(|entry| vec![entry.category.selected_by_default(); entry.targets.len()])
                    .collect();

                self.focus = self.focus.min(report.entries.len().saturating_sub(1));
                self.state = State::Done { report, selected };
            }
            Message::Toggle(entry, checked) => {
                if let State::Done { selected, .. } = &mut self.state
                    && let Some(items) = selected.get_mut(entry)
                {
                    items.fill(checked);
                }
            }
            Message::ToggleItem(entry, item, checked) => {
                if let State::Done { selected, .. } = &mut self.state
                    && let Some(slot) = selected
                        .get_mut(entry)
                        .and_then(|items| items.get_mut(item))
                {
                    *slot = checked;
                }
            }
            Message::SelectAll(checked) => {
                if let State::Done { selected, .. } = &mut self.state {
                    for items in selected {
                        items.fill(checked);
                    }
                }
            }
            Message::Focus(entry) => self.focus = entry,
            Message::Reveal(path) => platform::reveal(&path),
            Message::Clean => {
                if let State::Done { report, selected } = &self.state {
                    let paths = report.paths(selected);
                    self.state = State::Cleaning;

                    return Task::perform(
                        background(move || junk::clean(&paths)),
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
        let (task, running) = task::run(junk::scan, Message::Scanning);
        self.state = State::Scanning(running);
        task
    }

    pub fn summary(&self) -> Summary {
        match &self.state {
            State::Idle => Summary::Idle,
            State::Scanning(running) => Summary::Scanning(running.snapshot),
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
            State::Idle => vec![intro(self.focus)],
            State::Scanning(running) => vec![ui::progress(
                "Scanning junk locations",
                running.snapshot,
                Message::Stop,
            )],
            State::Cleaning => vec![ui::empty(
                Icon::Trash2,
                "Moving to Trash",
                "This only takes a moment.",
                space().into(),
            )],
            State::Done { report, selected } => results(report, selected, self.focus, self.cleanup),
        };

        let actions = rescan.map(|rescan| ui::with_metrics(self.performance, rescan));

        ui::page(TITLE, SUBTITLE, actions, body)
    }
}

fn intro<'a>(focus: usize) -> Element<'a, Message> {
    let mut sidebar = vec![container(ui::label("Categories")).padding([12, 12]).into()];

    sidebar.extend(
        Category::ALL
            .into_iter()
            .enumerate()
            .map(|(index, category)| {
                ui::choice(
                    None,
                    category.title(),
                    "—".to_owned(),
                    index == focus,
                    Message::Focus(index),
                )
            }),
    );

    let category = Category::ALL[focus.min(Category::ALL.len() - 1)];

    let header = column![
        text(category.title())
            .size(16)
            .font(style::MEDIUM)
            .style(style::strong),
        text(category.description()).size(13).style(style::muted),
        text(category.location()).size(12).style(style::muted),
    ]
    .spacing(6)
    .padding(20);

    let message = column![
        text("Not scanned yet")
            .size(15)
            .font(style::SEMIBOLD)
            .style(style::strong),
        text("Sizes show up here after a scan. Nothing is removed until you review it.")
            .size(13)
            .style(style::muted),
        space().height(10),
        ui::action(
            "Start scan",
            Some(Icon::Play),
            style::primary,
            Some(Message::Scan),
        ),
    ]
    .spacing(4)
    .align_x(Center);

    let content = column![header, ui::separator(), container(message).center(Fill)].into();

    let clean = ui::action("Clean up", Some(Icon::Trash2), style::primary, None);

    ui::split(sidebar, content, "Not scanned".to_owned(), clean)
}

fn results<'a>(
    report: &'a Report,
    selected: &'a [Vec<bool>],
    focus: usize,
    cleanup: Option<Cleanup>,
) -> Vec<Element<'a, Message>> {
    let (selected_count, selected_bytes) = report
        .entries
        .iter()
        .zip(selected)
        .flat_map(|(entry, items)| entry.targets.iter().zip(items))
        .filter(|&(_, &checked)| checked)
        .fold((0, 0), |(count, bytes), (target, _)| {
            (count + 1, bytes + target.bytes)
        });

    let all = selected.iter().flatten().all(|&checked| checked)
        && selected.iter().any(|items| !items.is_empty());

    let mut sidebar = vec![
        row![
            ui::check(all, Some(Message::SelectAll)),
            ui::label("Categories")
        ]
        .align_y(Center)
        .padding([12, 12])
        .into(),
    ];

    sidebar.extend(report.entries.iter().zip(selected).enumerate().map(
        |(index, (entry, items))| {
            let checked = !items.is_empty() && items.iter().all(|&checked| checked);

            ui::choice(
                Some(ui::check(
                    checked,
                    Some(move |checked| Message::Toggle(index, checked)),
                )),
                entry.category.title(),
                format::bytes(entry.usage.bytes),
                index == focus,
                Message::Focus(index),
            )
        },
    ));

    let selection = ui::selection(selected_count, selected_bytes);
    let status = match cleanup {
        Some(cleanup) => format!("{}  ·  {selection}", cleanup.summary()),
        None => selection,
    };

    let clean = ui::action(
        "Clean up",
        Some(Icon::Trash2),
        style::primary,
        (selected_count > 0).then_some(Message::Clean),
    );

    let content = match (report.entries.get(focus), selected.get(focus)) {
        (Some(entry), Some(items)) => details(focus, entry, items),
        _ => container(text("Nothing found").size(14).style(style::muted))
            .padding(20)
            .into(),
    };

    vec![ui::split(sidebar, content, status, clean)]
}

fn details<'a>(index: usize, entry: &'a Entry, selected: &'a [bool]) -> Element<'a, Message> {
    let category = entry.category;

    let header = column![
        row![
            text(category.title())
                .size(16)
                .font(style::MEDIUM)
                .style(style::strong),
            space().width(Fill),
            text(format::bytes(entry.usage.bytes))
                .size(16)
                .font(style::MEDIUM)
                .style(style::strong),
        ]
        .align_y(Center),
        text(category.description()).size(13).style(style::muted),
        text(format!(
            "{}  ·  {} files",
            category.location(),
            format::count(entry.usage.files)
        ))
        .size(12)
        .style(style::muted),
    ]
    .spacing(6)
    .padding(20);

    if entry.targets.is_empty() {
        return column![
            header,
            ui::separator(),
            container(text("Nothing to clean here.").size(14).style(style::muted)).padding(20),
        ]
        .into();
    }

    let all = selected.iter().all(|&checked| checked);

    let toolbar = row![
        ui::check(all, Some(move |checked| Message::Toggle(index, checked))),
        text(format!(
            "{} items",
            format::count(entry.targets.len() as u64)
        ))
        .size(13)
        .style(style::muted),
    ]
    .spacing(12)
    .align_y(Center)
    .padding([12, 20]);

    let mut items = Column::new();
    for (item, (target, &checked)) in entry.targets.iter().zip(selected).enumerate() {
        if item > 0 {
            items = items.push(ui::separator());
        }

        items = items.push(ui::line(
            Some(ui::check(
                checked,
                Some(move |checked| Message::ToggleItem(index, item, checked)),
            )),
            target.icon.as_ref(),
            target.bytes,
            target.name.as_str(),
            Message::Reveal(target.path.clone()),
        ));
    }

    column![
        header,
        ui::separator(),
        toolbar,
        ui::separator(),
        scrollable(items).height(Fill)
    ]
    .into()
}
