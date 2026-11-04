use std::cmp::Reverse;
use std::fmt;
use std::fs;
use std::mem;
use std::sync::Arc;

use iced::alignment::Horizontal;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{
    Row, button, canvas, column, container, mouse_area, pick_list, progress_bar, row, space, text,
};
use iced::{Center, Color, Element, Fill, FillPortion, Task};
use lucide_icons::Icon;

use crate::format;
use crate::recycle::{self, Cleanup};
use crate::scan::disk::Disk;
use crate::scan::explorer::{self, FileKind, Kind, Node, Tree};
use crate::scan::progress::Snapshot;
use crate::task::{self, Update, background};
use crate::ui::treemap::{self, Tile, Treemap};
use crate::ui::{self, Sort, Summary, style};

const TITLE: &str = "Disk Explorer";
const SUBTITLE: &str = "Browse your disk by size and pick what to remove.";
const VISIBLE_ROWS: usize = 300;
const SIZE_WIDTH: f32 = 88.0;
const SHARE_WIDTH: f32 = 96.0;
const COLUMN_GAP: f32 = 16.0;
const TREEMAP_TILES: usize = 60;
const PREVIEW_PARTS: usize = 24;
const CRUMB_CHARS: usize = 18;
const LAST_CRUMB_CHARS: usize = 32;

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
    choices: Vec<Choice>,
    hovered: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Choice {
    filter: Option<FileKind>,
    bytes: u64,
}

impl fmt::Display for Choice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = self.filter.map_or("All files", FileKind::title);
        write!(formatter, "{title}  ·  {}", format::bytes(self.bytes))
    }
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
    SelectAll(bool),
    Hover(Option<Vec<usize>>),
    Unhover(Vec<usize>),
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

        let body = match &self.state {
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

        let actions = rescan.map(|rescan| ui::with_metrics(self.performance, rescan));

        ui::page(TITLE, SUBTITLE, actions, body)
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
            choices: Vec::new(),
            hovered: None,
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
            Message::Hover(indices) => self.hovered = indices,
            Message::Unhover(indices) => {
                if self.hovered.as_ref() == Some(&indices) {
                    self.hovered = None;
                }
            }
            Message::SelectAll(checked) => {
                let targets: Vec<Vec<usize>> = self.selectable().cloned().collect();

                for target in &targets {
                    self.selected
                        .retain(|selected| !selected.starts_with(target));
                }
                if checked {
                    self.selected.extend(targets);
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

        let mut choices = vec![Choice {
            filter: None,
            bytes: current.bytes,
        }];
        choices.extend(
            FileKind::ALL
                .into_iter()
                .zip(breakdown)
                .filter(|&(kind, bytes)| bytes > 0 || self.filter == Some(kind))
                .map(|(kind, bytes)| Choice {
                    filter: Some(kind),
                    bytes,
                }),
        );

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

        self.choices = choices;
        self.rows = rows;
        self.hidden = hidden;
        self.hovered = None;
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

    fn selectable(&self) -> impl Iterator<Item = &Vec<usize>> {
        self.rows.iter().filter(|indices| {
            self.tree
                .node
                .find(indices)
                .is_some_and(|node| node.kind != Kind::Rest)
                && self.check_state(indices).1
        })
    }

    fn all_state(&self) -> (bool, bool) {
        let mut selectable = self.selectable().peekable();

        if selectable.peek().is_none() {
            return (self.covered(), false);
        }

        (
            selectable.all(|indices| self.selected.contains(indices)),
            true,
        )
    }

    fn covered(&self) -> bool {
        self.selected
            .iter()
            .any(|selected| self.path.starts_with(selected))
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
        let (all, can_select) = self.all_state();

        let head = row![
            ui::check(all, can_select.then_some(Message::SelectAll)),
            ui::sortable(
                "Size",
                SIZE_WIDTH,
                true,
                self.sort,
                SortKey::Size,
                Message::Sort(SortKey::Size),
            ),
            space().width(COLUMN_GAP),
            ui::heading("Share", SHARE_WIDTH, false),
            space().width(COLUMN_GAP),
            ui::sortable(
                "Name",
                Fill,
                false,
                self.sort,
                SortKey::Name,
                Message::Sort(SortKey::Name),
            ),
            space().width(28),
        ];

        let tiles = self.tiles();

        let mut rows: Vec<Element<'_, Message>> = self
            .rows
            .iter()
            .filter_map(|indices| {
                let node = self.tree.node.find(indices)?;
                let (checked, enabled) = self.check_state(indices);
                let color = tiles
                    .iter()
                    .position(|tile| tile.indices == indices.as_slice())
                    .filter(|_| node.kind != Kind::Rest)
                    .map(treemap::color);

                Some(entry(
                    indices,
                    node,
                    current.bytes,
                    Look {
                        checked,
                        enabled,
                        highlighted: self.hovered.as_ref() == Some(indices),
                        color,
                    },
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
            text(match cleanup {
                Some(cleanup) if selected_count == 0 => cleanup.summary(),
                _ => ui::selection(selected_count, selected_bytes),
            })
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

        let table = ui::table(
            container(self.toolbar()).padding([12, 16]).into(),
            head.into(),
            rows,
            footer.into(),
        );

        vec![
            row![
                container(table).width(FillPortion(3)),
                container(
                    canvas(Treemap {
                        tiles,
                        hovered: self.hovered.as_deref(),
                    })
                    .width(Fill)
                    .height(Fill),
                )
                .padding(8)
                .width(FillPortion(2))
                .height(Fill)
                .style(style::card),
            ]
            .spacing(16)
            .height(Fill)
            .into(),
        ]
    }

    fn tiles(&self) -> Vec<Tile<'_>> {
        let mut tiles: Vec<Tile<'_>> = self
            .rows
            .iter()
            .filter_map(|indices| {
                let node = self.tree.node.find(indices)?;
                (node.bytes > 0).then(|| Tile {
                    indices,
                    name: node.name(),
                    bytes: node.bytes,
                    kind: node.kind,
                    parts: node
                        .children
                        .iter()
                        .take(PREVIEW_PARTS)
                        .map(|child| child.bytes)
                        .filter(|&bytes| bytes > 0)
                        .collect(),
                })
            })
            .collect();

        tiles.sort_unstable_by_key(|tile| Reverse(tile.bytes));
        tiles.truncate(TREEMAP_TILES);
        tiles
    }

    fn toolbar(&self) -> Element<'_, Message> {
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

            let last = depth == self.path.len();
            let limit = if last { LAST_CRUMB_CHARS } else { CRUMB_CHARS };
            let label = text(ui::truncate(node.name(), limit))
                .size(14)
                .wrapping(Wrapping::None);

            crumbs = if last {
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
            container(crumbs).width(Fill).clip(true),
            space().width(8),
            pick_list(
                self.choices.as_slice(),
                self.choices
                    .iter()
                    .find(|choice| choice.filter == self.filter),
                |choice: Choice| Message::Filter(choice.filter),
            )
            .text_size(14)
            .padding([7, 12])
            .handle(pick_list::Handle::Static(pick_list::Icon {
                font: style::ICONS,
                code_point: Icon::ChevronDown.unicode(),
                size: Some(11.into()),
                line_height: LineHeight::default(),
                shaping: Shaping::Basic,
            }))
            .style(style::dropdown)
            .menu_style(style::menu),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    }
}

#[derive(Debug, Clone, Copy)]
struct Look {
    checked: bool,
    enabled: bool,
    highlighted: bool,
    color: Option<Color>,
}

fn entry<'a>(
    indices: &[usize],
    node: &'a Node,
    parent_bytes: u64,
    look: Look,
    location: Option<String>,
) -> Element<'a, Message> {
    let Look {
        checked,
        enabled,
        highlighted,
        color,
    } = look;

    let icon = match node.kind {
        Kind::Folder => Icon::Folder,
        Kind::File(_) => Icon::File,
        Kind::Rest => Icon::Layers,
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
        ui::icon(icon).size(16).color(color.unwrap_or(style::MUTED)),
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
        container(
            text(format::bytes(node.bytes))
                .size(14)
                .font(style::MEDIUM)
                .style(style::strong),
        )
        .width(SIZE_WIDTH)
        .align_x(Horizontal::Right),
        space().width(COLUMN_GAP),
        progress_bar(0.0..=1.0, share)
            .length(SHARE_WIDTH)
            .girth(4)
            .style(style::share),
        space().width(COLUMN_GAP),
        container(label).width(Fill).clip(true),
        container(if node.kind == Kind::Folder {
            ui::icon(Icon::ChevronRight).size(14).style(style::muted)
        } else {
            text("")
        })
        .width(28)
        .align_x(Horizontal::Right),
    ]
    .align_y(Center);

    let row: Element<'a, Message> = if node.kind == Kind::Folder {
        button(content)
            .padding([10, 16])
            .width(Fill)
            .style(style::row(highlighted))
            .on_press(Message::Open(indices.to_vec()))
            .into()
    } else {
        container(content)
            .padding([10, 16])
            .style(style::highlight(highlighted))
            .into()
    };

    mouse_area(row)
        .on_enter(Message::Hover(Some(indices.to_vec())))
        .on_exit(Message::Unhover(indices.to_vec()))
        .into()
}
