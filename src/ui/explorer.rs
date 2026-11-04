use std::fs;
use std::mem;
use std::sync::Arc;

use iced::alignment::Horizontal;
use iced::widget::text::Wrapping;
use iced::widget::{Row, button, column, container, progress_bar, row, space, text};
use iced::{Center, Element, Fill, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::disk::Disk;
use crate::scan::explorer::{self, Breakdown, FileKind, Kind, Node, Tree};
use crate::scan::progress::Snapshot;
use crate::task::{self, Update, background};
use crate::ui::{self, CHECK_WIDTH, Sort, Summary, style};

const TITLE: &str = "Disk Explorer";
const SUBTITLE: &str = "Browse your disk by size and pick what to remove.";
const VISIBLE_ROWS: usize = 300;

#[derive(Debug, Default)]
pub struct Explorer {
    state: State,
    disk: Option<Disk>,
    cleanup: Option<Cleanup>,
    performance: Option<Snapshot>,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Scanning(Snapshot),
    Removing(Browser),
    Done(Browser),
}

#[derive(Debug)]
struct Browser {
    tree: Tree,
    path: Vec<usize>,
    back: Vec<Vec<usize>>,
    forward: Vec<Vec<usize>>,
    selected: Vec<Vec<usize>>,
    filter: Option<FileKind>,
    sort: Sort<SortKey>,
    rows: Vec<Vec<usize>>,
    hidden: usize,
    breakdown: Breakdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Size,
}

#[derive(Debug, Clone)]
pub enum Message {
    Scan,
    Scanning(Update<Arc<Tree>>),
    Open(Vec<usize>),
    Back,
    Forward,
    Jump(usize),
    Toggle(Vec<usize>, bool),
    Filter(Option<FileKind>),
    Sort(SortKey),
    Remove,
    Removed(Cleanup),
}

impl Explorer {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scan => {
                self.cleanup = None;
                self.state = State::Scanning(Snapshot::default());

                return Task::run(
                    task::with_progress(|progress| Arc::new(explorer::scan(progress))),
                    Message::Scanning,
                );
            }
            Message::Scanning(Update::Progress(snapshot)) => {
                if let State::Scanning(current) = &mut self.state {
                    *current = snapshot;
                }
            }
            Message::Scanning(Update::Finished(tree, snapshot)) => {
                self.performance = Some(snapshot);
                let tree = Arc::try_unwrap(tree).unwrap_or_else(|shared| (*shared).clone());
                self.disk = Disk::root();
                self.state = State::Done(Browser::new(tree));
            }
            Message::Removed(cleanup) => {
                if let State::Removing(mut browser) = mem::take(&mut self.state) {
                    browser.prune();
                    self.state = State::Done(browser);
                }
                self.disk = Disk::root();
                self.cleanup = Some(cleanup);
            }
            Message::Remove => {
                if let State::Done(browser) = mem::take(&mut self.state) {
                    let paths: Vec<_> = browser
                        .selected
                        .iter()
                        .filter_map(|indices| browser.tree.path(indices))
                        .collect();

                    self.state = State::Removing(browser);

                    return Task::perform(
                        background(move || recycle::move_to_trash(&paths)),
                        Message::Removed,
                    );
                }
            }
            message => {
                if let State::Done(browser) = &mut self.state {
                    browser.update(message);
                }
            }
        }

        Task::none()
    }

    pub fn summary(&self) -> Summary {
        match &self.state {
            State::Idle => Summary::Idle,
            State::Scanning(snapshot) => Summary::Scanning(*snapshot),
            State::Removing(_) => Summary::Busy,
            State::Done(browser) => Summary::Found {
                items: browser.tree.node.items,
                bytes: browser.tree.node.bytes,
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let rescan = matches!(self.state, State::Done(_)).then(|| {
            ui::action(
                "Rescan",
                Some(Icon::RotateCw),
                style::secondary,
                Some(Message::Scan),
            )
        });

        let mut body = match &self.state {
            State::Idle => vec![ui::empty(
                Icon::HardDrive,
                "No scan yet",
                "Map every folder on your startup disk, largest first.",
                ui::action(
                    "Scan disk",
                    Some(Icon::Play),
                    style::primary,
                    Some(Message::Scan),
                ),
            )],
            State::Scanning(snapshot) => vec![ui::progress("Scanning your disk", *snapshot)],
            State::Removing(_) => vec![ui::empty(
                Icon::Trash2,
                "Moving to Trash",
                "This only takes a moment.",
                space().into(),
            )],
            State::Done(browser) => browser.view(self.disk, self.cleanup),
        };

        if let (State::Done(_), Some(snapshot)) = (&self.state, self.performance) {
            body.insert(0, ui::performance(snapshot));
        }

        ui::page(TITLE, SUBTITLE, rescan, body)
    }
}

impl Browser {
    fn new(tree: Tree) -> Self {
        let mut browser = Self {
            tree,
            path: Vec::new(),
            back: Vec::new(),
            forward: Vec::new(),
            selected: Vec::new(),
            filter: None,
            sort: Sort {
                key: SortKey::Size,
                descending: true,
            },
            rows: Vec::new(),
            hidden: 0,
            breakdown: Breakdown::default(),
        };
        browser.refresh();
        browser
    }

    fn current(&self) -> &Node {
        self.tree.node.find(&self.path).unwrap_or(&self.tree.node)
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Open(indices) => self.navigate(indices),
            Message::Jump(depth) => {
                if depth < self.path.len() {
                    self.navigate(self.path[..depth].to_vec());
                }
            }
            Message::Back => {
                if let Some(path) = self.back.pop() {
                    self.forward.push(mem::replace(&mut self.path, path));
                    self.refresh();
                }
            }
            Message::Forward => {
                if let Some(path) = self.forward.pop() {
                    self.back.push(mem::replace(&mut self.path, path));
                    self.refresh();
                }
            }
            Message::Toggle(target, checked) => {
                self.selected
                    .retain(|selected| !selected.starts_with(&target));
                if checked {
                    self.selected.push(target);
                }
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.refresh();
            }
            Message::Sort(key) => {
                self.sort = self.sort.toggle(key, key == SortKey::Size);
                self.refresh();
            }
            _ => {}
        }
    }

    fn navigate(&mut self, path: Vec<usize>) {
        self.back.push(mem::replace(&mut self.path, path));
        self.forward.clear();
        self.refresh();
    }

    fn refresh(&mut self) {
        let current = self.current();
        let breakdown = current.breakdown();

        let mut rows: Vec<Vec<usize>> = match self.filter {
            None => (0..current.children.len())
                .map(|index| {
                    let mut indices = self.path.clone();
                    indices.push(index);
                    indices
                })
                .collect(),
            Some(kind) => {
                let mut found = Vec::new();
                current.files_of(kind, &mut self.path.clone(), &mut found);
                found
            }
        };

        let node = |indices: &Vec<usize>| self.tree.node.find(indices);

        match self.sort.key {
            SortKey::Size => {
                rows.sort_unstable_by_key(|indices| node(indices).map_or(0, |node| node.bytes))
            }
            SortKey::Name => rows.sort_by_cached_key(|indices| {
                node(indices)
                    .map(|node| node.name().to_lowercase())
                    .unwrap_or_default()
            }),
        }

        if self.sort.descending {
            rows.reverse();
        }

        let hidden = rows.len().saturating_sub(VISIBLE_ROWS);
        rows.truncate(VISIBLE_ROWS);

        self.breakdown = breakdown;
        self.rows = rows;
        self.hidden = hidden;
    }

    fn prune(&mut self) {
        let mut selected = mem::take(&mut self.selected);
        selected.sort_unstable_by(|a, b| b.cmp(a));

        for indices in selected {
            if let Some(path) = self.tree.path(&indices)
                && fs::symlink_metadata(&path).is_err()
            {
                self.tree.remove(&indices);
            }
        }

        self.path.clear();
        self.back.clear();
        self.forward.clear();
        self.refresh();
    }

    fn check_state(&self, indices: &[usize]) -> (bool, bool) {
        let covered = self
            .selected
            .iter()
            .any(|selected| selected.len() < indices.len() && indices.starts_with(selected));
        let selected = self.selected.iter().any(|selected| selected == indices);

        (covered || selected, !covered)
    }

    fn selection(&self) -> (usize, u64) {
        self.selected
            .iter()
            .filter_map(|indices| self.tree.node.find(indices))
            .fold((0, 0), |(count, bytes), node| {
                (count + 1, bytes + node.bytes)
            })
    }

    fn location(&self, indices: &[usize]) -> Option<String> {
        self.filter?;

        let mut node = self.current();
        let mut parts = Vec::new();

        for &index in indices.get(self.path.len()..indices.len().saturating_sub(1))? {
            node = node.children.get(index)?;
            parts.push(node.name());
        }

        Some(if parts.is_empty() {
            "in this folder".to_owned()
        } else {
            format!("in {}", parts.join("/"))
        })
    }

    fn view(&self, disk: Option<Disk>, cleanup: Option<Cleanup>) -> Vec<Element<'_, Message>> {
        let current = self.current();
        let (selected_count, selected_bytes) = self.selection();

        let head = row![
            space().width(CHECK_WIDTH),
            ui::sortable(
                "Name",
                Fill,
                false,
                self.sort,
                SortKey::Name,
                Message::Sort(SortKey::Name),
            ),
            ui::heading("Share", 160, false),
            ui::sortable(
                "Size",
                120,
                true,
                self.sort,
                SortKey::Size,
                Message::Sort(SortKey::Size),
            ),
            space().width(28),
        ];

        let mut rows: Vec<Element<'_, Message>> = self
            .rows
            .iter()
            .filter_map(|indices| {
                let node = self.tree.node.find(indices)?;
                let (checked, enabled) = self.check_state(indices);

                Some(entry(
                    indices,
                    node,
                    current.bytes,
                    checked,
                    enabled,
                    self.location(indices),
                ))
            })
            .collect();

        if self.hidden > 0 {
            rows.push(
                container(
                    text(format!(
                        "{} more items not shown",
                        format::count(self.hidden as u64)
                    ))
                    .size(13)
                    .style(style::muted),
                )
                .padding([10, 16])
                .into(),
            );
        }

        let mut usage = column![].spacing(6);
        if let Some(disk) = disk {
            usage = usage
                .push(
                    text(format!(
                        "Startup Disk  ·  {} of {} used",
                        format::bytes(disk.used()),
                        format::bytes(disk.total)
                    ))
                    .size(13)
                    .style(style::muted),
                )
                .push(
                    progress_bar(0.0..=1.0, disk.fraction())
                        .length(240)
                        .girth(4)
                        .style(style::progress),
                );
        }

        let footer = row![
            usage,
            space().width(Fill),
            text(ui::selection(selected_count, selected_bytes))
                .size(14)
                .style(style::muted),
            ui::action(
                "Move to Trash",
                Some(Icon::Trash2),
                style::primary,
                (selected_count > 0).then_some(Message::Remove),
            ),
        ]
        .spacing(16)
        .align_y(Center);

        let description = cleanup.map_or_else(
            || {
                format!(
                    "{}  ·  {} items",
                    format::bytes(current.bytes),
                    format::count(current.items)
                )
            },
            Cleanup::summary,
        );

        let header = column![self.toolbar(description), self.categories()].spacing(12);

        vec![ui::table(
            container(header).padding([12, 16]).into(),
            head.into(),
            rows,
            footer.into(),
        )]
    }

    fn categories(&self) -> Element<'_, Message> {
        let chip = |label: String, filter: Option<FileKind>| -> Element<'_, Message> {
            button(text(label).size(13).font(style::MEDIUM))
                .padding([4, 10])
                .style(style::nav(self.filter == filter))
                .on_press(Message::Filter(filter))
                .into()
        };

        let mut chips = vec![chip("All".to_owned(), None)];

        for kind in FileKind::ALL {
            let bytes = self.breakdown[kind as usize];
            if bytes > 0 || self.filter == Some(kind) {
                chips.push(chip(
                    format!("{}  {}", kind.title(), format::bytes(bytes)),
                    Some(kind),
                ));
            }
        }

        Row::with_children(chips).spacing(4).wrap().into()
    }

    fn toolbar(&self, description: String) -> Element<'_, Message> {
        let mut crumbs = Row::new().spacing(4).align_y(Center);
        let mut node = &self.tree.node;

        for depth in 0..=self.path.len() {
            if depth > 0 {
                let Some(child) = node.children.get(self.path[depth - 1]) else {
                    break;
                };
                node = child;
                crumbs = crumbs.push(ui::icon(Icon::ChevronRight).size(12).style(style::muted));
            }

            let label = text(node.name()).size(14).wrapping(Wrapping::None);

            crumbs = if depth == self.path.len() {
                crumbs.push(label.font(style::MEDIUM).style(style::strong))
            } else {
                crumbs.push(
                    button(label)
                        .padding([2, 4])
                        .style(style::link)
                        .on_press(Message::Jump(depth)),
                )
            };
        }

        row![
            ui::icon_button(
                Icon::ChevronLeft,
                (!self.back.is_empty()).then_some(Message::Back),
            ),
            ui::icon_button(
                Icon::ChevronRight,
                (!self.forward.is_empty()).then_some(Message::Forward),
            ),
            space().width(8),
            crumbs,
            space().width(Fill),
            text(description).size(14).style(style::muted),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    }
}

fn entry<'a>(
    indices: &[usize],
    node: &'a Node,
    parent_bytes: u64,
    checked: bool,
    enabled: bool,
    location: Option<String>,
) -> Element<'a, Message> {
    let (icon, icon_style): (Icon, fn(&iced::Theme) -> text::Style) = match node.kind {
        Kind::Folder => (Icon::Folder, style::accent),
        Kind::File(_) => (Icon::File, style::muted),
        Kind::Rest => (Icon::Layers, style::muted),
    };

    let toggle = (enabled && node.kind != Kind::Rest).then(|| {
        let indices = indices.to_vec();
        move |checked| Message::Toggle(indices.clone(), checked)
    });

    let share = if parent_bytes == 0 {
        0.0
    } else {
        node.bytes as f32 / parent_bytes as f32
    };

    let mut label = row![
        ui::icon(icon).size(16).style(icon_style),
        ui::name(node.name())
    ]
    .spacing(10)
    .align_y(Center)
    .width(Fill);

    if let Some(location) = location {
        label = label.push(
            text(location)
                .size(13)
                .style(style::muted)
                .wrapping(Wrapping::None),
        );
    }

    let content = row![
        ui::check(checked, toggle),
        container(label).width(Fill).clip(true),
        container(
            progress_bar(0.0..=1.0, share)
                .length(120)
                .girth(4)
                .style(style::share),
        )
        .width(160),
        ui::value(format::bytes(node.bytes), 120),
        container(if node.kind == Kind::Folder {
            ui::icon(Icon::ChevronRight).size(14).style(style::muted)
        } else {
            text("")
        })
        .width(28)
        .align_x(Horizontal::Right),
    ]
    .align_y(Center);

    if node.kind == Kind::Folder {
        button(content)
            .padding([10, 16])
            .width(Fill)
            .style(style::row)
            .on_press(Message::Open(indices.to_vec()))
            .into()
    } else {
        container(content).padding([10, 16]).into()
    }
}
