use std::path::PathBuf;

use iced::widget::{Column, Row, button, column, row, scrollable, space, text};
use iced::{Center, Element, Fill, Shrink, Task};
use lucide_icons::Icon;

use crate::recycle::{self, Cleanup};
use crate::scan::apps::{self, InstalledApp};
use crate::scan::progress::Snapshot;
use crate::task::{self, Running, Update, background};
use crate::ui::style;
use crate::ui::{self, Sort, Summary};
use crate::{format, platform};

const ROW_ICON: f32 = 32.0;
const DETAIL_ICON: f32 = 44.0;
const TITLE: &str = "Applications";
const SUBTITLE: &str = "Remove apps along with the files they leave behind.";

#[derive(Debug, Default)]
pub struct Uninstaller {
    state: State,
    sort: Sort<SortKey>,
    filter: Filter,
    open: Option<usize>,
    cleanup: Option<Cleanup>,
    performance: Option<Snapshot>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Leftovers,
    Selected,
}

impl Filter {
    fn title(self) -> &'static str {
        match self {
            Filter::All => "All applications",
            Filter::Leftovers => "With leftovers",
            Filter::Selected => "Selected",
        }
    }

    fn matches(self, app: &InstalledApp, selected: bool) -> bool {
        match self {
            Filter::All => true,
            Filter::Leftovers => !app.leftovers.is_empty(),
            Filter::Selected => selected,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortKey {
    #[default]
    Name,
    Leftovers,
    Size,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Scanning(Running),
    Removing,
    Done {
        apps: Vec<InstalledApp>,
        selected: Vec<bool>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanning(Update<Vec<InstalledApp>>),
    Stop,
    Toggle(usize, bool),
    SelectAll(bool),
    Filter(Filter),
    Open(usize),
    Close,
    Reveal(PathBuf),
    Sort(SortKey),
    Uninstall,
    Uninstalled(Cleanup),
}

impl Uninstaller {
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
            Message::Scanning(Update::Finished(apps, snapshot)) => {
                self.performance = Some(snapshot);

                let selected = vec![false; apps.len()];
                self.open = None;
                self.state = State::Done { apps, selected };
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.open = None;
            }
            Message::Open(index) => self.open = Some(index),
            Message::Close => self.open = None,
            Message::Reveal(path) => platform::reveal(&path),
            Message::Sort(key) => {
                self.sort = self.sort.toggle(key, key != SortKey::Name);
            }
            Message::SelectAll(checked) => {
                if let State::Done { apps, selected } = &mut self.state {
                    for (app, slot) in apps.iter().zip(selected.iter_mut()) {
                        if self.filter.matches(app, *slot) {
                            *slot = checked;
                        }
                    }
                }
            }
            Message::Toggle(index, checked) => {
                if let State::Done { selected, .. } = &mut self.state
                    && let Some(slot) = selected.get_mut(index)
                {
                    *slot = checked;
                }
            }
            Message::Uninstall => {
                if let State::Done { apps, selected } = &self.state {
                    let paths: Vec<PathBuf> = apps
                        .iter()
                        .zip(selected)
                        .filter(|&(_, &checked)| checked)
                        .flat_map(|(app, _)| app.paths().cloned())
                        .collect();

                    self.state = State::Removing;

                    return Task::perform(
                        background(move || recycle::move_to_trash(&paths)),
                        Message::Uninstalled,
                    );
                }
            }
            Message::Uninstalled(cleanup) => {
                self.cleanup = Some(cleanup);
                return self.start_scan();
            }
        }

        Task::none()
    }

    fn start_scan(&mut self) -> Task<Message> {
        let (task, running) = task::run(apps::scan, Message::Scanning);
        self.state = State::Scanning(running);
        task
    }

    pub fn summary(&self) -> Summary {
        match &self.state {
            State::Idle => Summary::Idle,
            State::Scanning(running) => Summary::Scanning(running.snapshot),
            State::Removing => Summary::Busy,
            State::Done { apps, .. } => Summary::Found {
                items: apps.len() as u64,
                bytes: apps.iter().map(InstalledApp::total_bytes).sum(),
            },
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
                Icon::Package,
                "No scan yet",
                "Find installed apps and everything they store in your Library.",
                ui::action(
                    "Find apps",
                    Some(Icon::Play),
                    style::primary,
                    Some(Message::Scan),
                ),
            )],
            State::Scanning(running) => vec![ui::progress(
                "Looking for apps",
                running.snapshot,
                Message::Stop,
            )],
            State::Removing => vec![ui::empty(
                Icon::Trash2,
                "Moving apps to Trash",
                "This only takes a moment.",
                space().into(),
            )],
            State::Done { apps, selected } => results(apps, selected, self),
        };

        let actions = rescan.map(|rescan| ui::with_metrics(self.performance, rescan));

        ui::page(TITLE, SUBTITLE, actions, body)
    }
}

fn results<'a>(
    apps: &'a [InstalledApp],
    selected: &'a [bool],
    view: &'a Uninstaller,
) -> Vec<Element<'a, Message>> {
    let (selected_count, selected_bytes) = apps
        .iter()
        .zip(selected)
        .filter(|&(_, &checked)| checked)
        .fold((0, 0), |(count, bytes), (app, _)| {
            (count + 1, bytes + app.total_bytes())
        });

    let sidebar = [
        ui::group("Show"),
        filter(view.filter, Filter::All, apps.len()),
        filter(
            view.filter,
            Filter::Leftovers,
            apps.iter().filter(|app| !app.leftovers.is_empty()).count(),
        ),
        filter(view.filter, Filter::Selected, selected_count),
    ];

    let content = match view.open.and_then(|index| apps.get(index)) {
        Some(app) => details(app),
        None => list(apps, selected, view.filter, view.sort),
    };

    let selection = ui::selection(selected_count, selected_bytes);
    let status = match view.cleanup {
        Some(cleanup) => format!("{}  ·  {selection}", cleanup.summary()),
        None => selection,
    };

    let uninstall = ui::action(
        "Uninstall",
        Some(Icon::Trash2),
        style::danger,
        (selected_count > 0).then_some(Message::Uninstall),
    );

    vec![ui::split(sidebar, content, status, uninstall)]
}

fn filter<'a>(current: Filter, filter: Filter, count: usize) -> Element<'a, Message> {
    ui::choice(
        None,
        filter.title(),
        format::count(count as u64),
        current == filter,
        Message::Filter(filter),
    )
}

fn list<'a>(
    apps: &'a [InstalledApp],
    selected: &'a [bool],
    filter: Filter,
    sort: Sort<SortKey>,
) -> Element<'a, Message> {
    let mut order: Vec<usize> = (0..apps.len())
        .filter(|&index| filter.matches(&apps[index], selected[index]))
        .collect();

    match sort.key {
        SortKey::Name => order.sort_by_cached_key(|&index| apps[index].name.to_lowercase()),
        SortKey::Leftovers => order.sort_by_key(|&index| apps[index].leftover_bytes),
        SortKey::Size => order.sort_by_key(|&index| apps[index].total_bytes()),
    }
    if sort.descending {
        order.reverse();
    }

    let all = !order.is_empty() && order.iter().all(|&index| selected[index]);

    let sorting = [
        ("Name", SortKey::Name),
        ("Size", SortKey::Size),
        ("Leftovers", SortKey::Leftovers),
    ]
    .map(|(label, key)| ui::sortable(label, Shrink, false, sort, key, Message::Sort(key)));

    let toolbar = row![
        ui::check(all, Some(Message::SelectAll)),
        text(format!("{} apps", format::count(order.len() as u64)))
            .size(13)
            .style(style::muted),
        space().width(Fill),
        text("Sort by").size(12).style(style::muted),
        Row::with_children(sorting).spacing(14),
    ]
    .spacing(12)
    .align_y(Center)
    .padding([12, 20]);

    let mut rows = Column::new();
    for (position, index) in order.into_iter().enumerate() {
        if position > 0 {
            rows = rows.push(ui::separator());
        }

        let app = &apps[index];
        let leftovers = match app.leftovers.len() {
            0 => "No leftovers".to_owned(),
            1 => format!("1 leftover  ·  {}", format::bytes(app.leftover_bytes)),
            count => format!(
                "{count} leftovers  ·  {}",
                format::bytes(app.leftover_bytes)
            ),
        };

        let content = row![
            ui::check(
                selected[index],
                Some(move |checked| Message::Toggle(index, checked)),
            ),
            ui::thumbnail(app.icon.as_ref(), ROW_ICON),
            column![
                ui::name(app.name.as_str()),
                text(leftovers).size(12).style(style::muted),
            ]
            .spacing(2)
            .width(Fill),
            text(format::bytes(app.total_bytes())).size(14),
            ui::icon(Icon::ChevronRight).size(14).style(style::muted),
        ]
        .spacing(12)
        .align_y(Center);

        rows = rows.push(
            button(content)
                .padding([12, 20])
                .width(Fill)
                .style(style::row(false))
                .on_press(Message::Open(index)),
        );
    }

    column![toolbar, ui::separator(), scrollable(rows).height(Fill)].into()
}

fn details(app: &InstalledApp) -> Element<'_, Message> {
    let back = button(
        row![
            ui::icon(Icon::ChevronLeft).size(14),
            text("All apps").size(13)
        ]
        .spacing(4)
        .align_y(Center),
    )
    .padding(0)
    .style(style::link)
    .on_press(Message::Close);

    let header = column![
        back,
        space().height(6),
        row![
            ui::thumbnail(app.icon.as_ref(), DETAIL_ICON),
            column![
                text(app.name.as_str())
                    .size(16)
                    .font(style::MEDIUM)
                    .style(style::strong),
                text(format!(
                    "{}  ·  {}",
                    format::bytes(app.total_bytes()),
                    ui::home_relative(&app.path)
                ))
                .size(12)
                .style(style::muted),
            ]
            .spacing(4)
            .width(Fill),
            ui::action(
                "Reveal in Finder",
                Some(Icon::FolderSearch),
                style::secondary,
                Some(Message::Reveal(app.path.clone())),
            ),
        ]
        .spacing(14)
        .align_y(Center),
    ]
    .spacing(6)
    .padding(20);

    let mut items = column![ui::line(
        None,
        app.icon.as_ref(),
        app.bytes,
        "App bundle",
        Message::Reveal(app.path.clone()),
    )];
    for leftover in &app.leftovers {
        items = items.push(ui::separator()).push(ui::line(
            None,
            None,
            leftover.bytes,
            ui::home_relative(&leftover.path),
            Message::Reveal(leftover.path.clone()),
        ));
    }

    column![header, ui::separator(), scrollable(items).height(Fill)].into()
}
