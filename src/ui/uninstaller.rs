use std::path::PathBuf;

use iced::widget::{row, space, text};
use iced::{Center, Element, Fill, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::apps::{self, InstalledApp};
use crate::scan::progress::Snapshot;
use crate::task::{self, Update, background};
use crate::ui::style::{self, Tone};
use crate::ui::{self, CHECK_WIDTH, Sort, Summary};

const TITLE: &str = "Applications";
const SUBTITLE: &str = "Remove apps along with the files they leave behind.";

#[derive(Debug, Default)]
pub struct Uninstaller {
    state: State,
    sort: Sort<SortKey>,
    cleanup: Option<Cleanup>,
    performance: Option<Snapshot>,
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
    Scanning(Snapshot),
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
    Toggle(usize, bool),
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
                if let State::Scanning(current) = &mut self.state {
                    *current = snapshot;
                }
            }
            Message::Scanning(Update::Finished(apps, snapshot)) => {
                self.performance = Some(snapshot);

                let selected = vec![false; apps.len()];
                self.state = State::Done { apps, selected };
            }
            Message::Sort(key) => {
                self.sort = self.sort.toggle(key, key != SortKey::Name);
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
        self.state = State::Scanning(Snapshot::default());
        Task::run(task::with_progress(apps::scan), Message::Scanning)
    }

    pub fn summary(&self) -> Summary {
        match &self.state {
            State::Idle => Summary::Idle,
            State::Scanning(snapshot) => Summary::Scanning(*snapshot),
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

        let mut body = match &self.state {
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
            State::Scanning(snapshot) => vec![ui::progress("Looking for apps", *snapshot)],
            State::Removing => vec![ui::empty(
                Icon::Trash2,
                "Moving apps to Trash",
                "This only takes a moment.",
                space().into(),
            )],
            State::Done { apps, selected } => results(apps, selected, self.sort, self.cleanup),
        };

        if let (State::Done { .. }, Some(snapshot)) = (&self.state, self.performance) {
            body.insert(0, ui::performance(snapshot));
        }

        ui::page(TITLE, SUBTITLE, rescan, body)
    }
}

fn results<'a>(
    apps: &'a [InstalledApp],
    selected: &'a [bool],
    sort: Sort<SortKey>,
    cleanup: Option<Cleanup>,
) -> Vec<Element<'a, Message>> {
    let total: u64 = apps.iter().map(InstalledApp::total_bytes).sum();

    let (selected_count, selected_bytes) = apps
        .iter()
        .zip(selected)
        .filter(|&(_, &checked)| checked)
        .fold((0, 0), |(count, bytes), (app, _)| {
            (count + 1, bytes + app.total_bytes())
        });

    let leftovers: u64 = apps.iter().map(|app| app.leftover_bytes).sum();
    let bundles = total - leftovers;

    let caption = cleanup.map_or_else(
        || "Apple apps are hidden so they can't be removed by accident.".to_owned(),
        Cleanup::summary,
    );

    let legend = row![
        ui::dot(Some(Tone::Orange)),
        text(format!("Apps  {}", format::bytes(bundles)))
            .size(13)
            .style(style::muted),
        space().width(12),
        ui::dot(Some(Tone::Blue)),
        text(format!("Leftovers  {}", format::bytes(leftovers)))
            .size(13)
            .style(style::muted),
    ]
    .spacing(8)
    .align_y(Center);

    let header = ui::total(format::bytes(total), "used by apps", caption)
        .push(space().height(6))
        .push(ui::segments([
            (bundles, Some(Tone::Orange)),
            (leftovers, Some(Tone::Blue)),
        ]))
        .push(legend)
        .spacing(8)
        .padding(16);

    let head = row![
        space().width(CHECK_WIDTH),
        ui::sortable(
            "Name",
            Fill,
            false,
            sort,
            SortKey::Name,
            Message::Sort(SortKey::Name),
        ),
        ui::sortable(
            "Leftovers",
            140,
            true,
            sort,
            SortKey::Leftovers,
            Message::Sort(SortKey::Leftovers),
        ),
        ui::sortable(
            "Size",
            120,
            true,
            sort,
            SortKey::Size,
            Message::Sort(SortKey::Size),
        ),
    ];

    let mut order: Vec<usize> = (0..apps.len()).collect();
    match sort.key {
        SortKey::Name => order.sort_by_cached_key(|&index| apps[index].name.to_lowercase()),
        SortKey::Leftovers => order.sort_by_key(|&index| apps[index].leftover_bytes),
        SortKey::Size => order.sort_by_key(|&index| apps[index].total_bytes()),
    }
    if sort.descending {
        order.reverse();
    }

    let rows = order
        .into_iter()
        .filter_map(|index| Some((index, apps.get(index)?, *selected.get(index)?)))
        .map(|(index, app, checked)| {
            let leftovers = match app.leftovers.len() {
                0 => "—".to_owned(),
                _ => format::bytes(app.leftover_bytes),
            };

            row![
                ui::check(
                    checked,
                    Some(move |checked| Message::Toggle(index, checked)),
                ),
                ui::name(app.name.as_str()),
                ui::value(leftovers, 140),
                ui::value(format::bytes(app.total_bytes()), 120),
            ]
            .align_y(Center)
            .padding([10, 16])
            .into()
        });

    let footer = row![
        text(ui::selection(selected_count, selected_bytes))
            .size(14)
            .style(style::muted),
        space().width(Fill),
        ui::action(
            "Uninstall",
            Some(Icon::Trash2),
            style::danger,
            (selected_count > 0).then_some(Message::Uninstall),
        ),
    ]
    .align_y(Center);

    vec![ui::table(header.into(), head.into(), rows, footer.into())]
}
