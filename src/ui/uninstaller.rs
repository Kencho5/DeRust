use std::path::PathBuf;

use iced::{Element, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::apps::{self, InstalledApp};
use crate::scan::progress::Snapshot;
use crate::task::{self, Update, background};
use crate::ui::{self, style};

const TITLE: &str = "Uninstaller";
const ICON: Icon = Icon::PackageX;

#[derive(Debug, Default)]
pub struct Uninstaller {
    state: State,
    cleanup: Option<Cleanup>,
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
            Message::Scanning(Update::Finished(apps)) => {
                let selected = vec![false; apps.len()];
                self.state = State::Done { apps, selected };
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

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            State::Idle => ui::prompt(
                ICON,
                TITLE,
                "Remove apps along with the files they leave behind.",
                Some(Message::Scan),
            ),
            State::Scanning(snapshot) => {
                ui::scanning(ICON, TITLE, "Looking for apps...", *snapshot)
            }
            State::Removing => ui::prompt(ICON, TITLE, "Moving apps to Trash...", None),
            State::Done { apps, selected } => results(apps, selected, self.cleanup),
        }
    }
}

fn results<'a>(
    apps: &'a [InstalledApp],
    selected: &'a [bool],
    cleanup: Option<Cleanup>,
) -> Element<'a, Message> {
    let (selected_count, selected_bytes) = apps
        .iter()
        .zip(selected)
        .filter(|&(_, &checked)| checked)
        .fold((0, 0), |(count, bytes), (app, _)| {
            (count + 1, bytes + app.total_bytes())
        });

    let rows = apps
        .iter()
        .zip(selected)
        .enumerate()
        .map(|(index, (app, &checked))| {
            let details = match app.leftovers.len() {
                0 => "No leftovers".to_owned(),
                1 => format!("1 leftover, {}", format::bytes(app.leftover_bytes)),
                count => format!("{count} leftovers, {}", format::bytes(app.leftover_bytes)),
            };

            ui::item(
                checked,
                move |checked| Message::Toggle(index, checked),
                Icon::AppWindow,
                app.name.as_str(),
                details,
                app.total_bytes(),
            )
        });

    let footer = ui::footer(
        ui::selection(selected_count, selected_bytes),
        [
            ui::action("Start Over", style::ghost, Some(Message::Scan)),
            ui::action(
                "Uninstall",
                style::remove,
                (selected_count > 0).then_some(Message::Uninstall),
            ),
        ],
    );

    ui::page(
        ICON,
        TITLE,
        format!("{} apps installed", apps.len()),
        cleanup.map(Cleanup::summary),
        rows,
        footer,
    )
}
