use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;

use iced::alignment::Horizontal;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{
    Column, Row, button, canvas, column, container, mouse_area, pick_list, progress_bar, row, rule,
    scrollable, space, text,
};
use iced::{Center, Color, Element, Fill, FillPortion, Shrink, Task};
use lucide_icons::Icon;
use rayon::prelude::*;

use crate::format;
use crate::platform::{self, FileIcon};
use crate::recycle::{self, Cleanup};
use crate::scan::disk::Disk;
use crate::scan::explorer::{self, FileKind, Kind, Node, Tree};
use crate::scan::progress::Snapshot;
use crate::task::{self, Running, Update, background};
use crate::ui::donut::{self, Donut, Slice};
use crate::ui::{self, Sort, Summary, style};

const TITLE: &str = "Disk Explorer";
const SUBTITLE: &str = "Browse your disk by size and pick what to remove.";
const VISIBLE_ROWS: usize = 300;
const SLICES: usize = 8;
const CHART_HEIGHT: f32 = 240.0;
const LEGEND_CHARS: usize = 32;
const CRUMB_CHARS: usize = 18;
const LAST_CRUMB_CHARS: usize = 32;
const REVEAL_WIDTH: f32 = 32.0;
const ROW_ICON: f32 = 28.0;
const SIZE_WIDTH: f32 = 80.0;
const CHEVRON_WIDTH: f32 = 16.0;

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
    Scanning(Running),
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
    icons: HashMap<Vec<usize>, Option<FileIcon>>,
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
    Stop,
    Open(Vec<usize>),
    Back,
    Forward,
    Jump(usize),
    Toggle(Vec<usize>, bool),
    SelectAll(bool),
    Hover(Option<Vec<usize>>),
    Unhover(Vec<usize>),
    Reveal(Vec<usize>),
    Icons(Vec<(Vec<usize>, FileIcon)>),
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

                let (task, running) = task::run(
                    |progress| Arc::new(explorer::scan(progress)),
                    Message::Scanning,
                );
                self.state = State::Scanning(running);
                return task;
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
            Message::Scanning(Update::Finished(tree, snapshot)) => {
                self.performance = Some(snapshot);
                let tree = Arc::try_unwrap(tree).unwrap_or_else(|shared| (*shared).clone());
                self.disk = Disk::root();

                let mut browser = Browser::new(tree);
                let icons = browser.load_icons();
                self.state = State::Done(browser);
                return icons;
            }
            Message::Removed(cleanup) => {
                self.disk = Disk::root();
                self.cleanup = Some(cleanup);

                if let State::Removing(mut browser) = mem::take(&mut self.state) {
                    browser.prune();
                    let icons = browser.load_icons();
                    self.state = State::Done(browser);
                    return icons;
                }
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
                    return browser.load_icons();
                }
            }
        }

        Task::none()
    }

    pub fn summary(&self) -> Summary {
        match &self.state {
            State::Idle => Summary::Idle,
            State::Scanning(running) => Summary::Scanning(running.snapshot),
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
            State::Scanning(running) => vec![ui::progress(
                "Scanning your disk",
                running.snapshot,
                Message::Stop,
            )],
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
            icons: HashMap::new(),
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
            Message::Icons(icons) => {
                for (indices, icon) in icons {
                    if let Some(slot) = self.icons.get_mut(&indices) {
                        *slot = Some(icon);
                    }
                }
            }
            Message::Reveal(indices) => {
                if let Some(path) = self.tree.path(&indices) {
                    platform::reveal(&path);
                }
            }
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

    fn load_icons(&mut self) -> Task<Message> {
        let missing: Vec<(Vec<usize>, PathBuf)> = self
            .rows
            .iter()
            .filter(|indices| !self.icons.contains_key(*indices))
            .filter(|indices| {
                self.tree
                    .node
                    .find(indices)
                    .is_some_and(|node| node.kind != Kind::Rest)
            })
            .filter_map(|indices| Some((indices.clone(), self.tree.path(indices)?)))
            .collect();

        if missing.is_empty() {
            return Task::none();
        }

        for (indices, _) in &missing {
            self.icons.insert(indices.clone(), None);
        }

        Task::perform(
            background(move || {
                missing
                    .into_par_iter()
                    .filter_map(|(indices, path)| Some((indices, platform::icon(&path)?)))
                    .collect()
            }),
            Message::Icons,
        )
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
        self.icons.clear();
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
        let (selected_count, selected_bytes) = self.selection();
        let (all, can_select) = self.all_state();

        let sorting = [("Size", SortKey::Size), ("Name", SortKey::Name)].map(|(label, key)| {
            ui::sortable(label, Shrink, false, self.sort, key, Message::Sort(key))
        });

        let shown = format::count((self.rows.len() + self.hidden) as u64);
        let head = row![
            ui::check(all, can_select.then_some(Message::SelectAll)),
            text(format!("{shown} items")).size(13).style(style::muted),
            space().width(Fill),
            text("Sort by").size(12).style(style::muted),
            Row::with_children(sorting).spacing(14),
        ]
        .spacing(12)
        .align_y(Center)
        .padding([10, 20]);

        let slices = self.slices();

        let mut rows = Column::new();
        for (position, entry) in self
            .rows
            .iter()
            .filter_map(|indices| {
                let node = self.tree.node.find(indices)?;
                let (checked, enabled) = self.check_state(indices);
                let color = slices
                    .iter()
                    .find(|slice| slice.indices == Some(indices.as_slice()))
                    .map(|slice| slice.color);

                Some(entry(
                    indices,
                    node,
                    Look {
                        checked,
                        enabled,
                        highlighted: self.hovered.as_ref() == Some(indices),
                        color,
                        icon: self.icons.get(indices).and_then(Option::as_ref),
                    },
                    self.location(indices),
                ))
            })
            .enumerate()
        {
            if position > 0 {
                rows = rows.push(ui::separator());
            }
            rows = rows.push(entry);
        }

        if self.hidden > 0 {
            rows = rows.push(ui::separator()).push(
                container(
                    text(format!(
                        "{} more items not shown",
                        format::count(self.hidden as u64)
                    ))
                    .size(13)
                    .style(style::muted),
                )
                .padding([12, 20]),
            );
        }

        let list = column![
            container(self.toolbar()).padding([12, 20]),
            ui::separator(),
            head,
            ui::separator(),
            scrollable(rows).height(Fill),
        ]
        .width(FillPortion(3));

        let breakdown = self.breakdown(slices);

        let mut usage = row![].spacing(12).align_y(Center);
        if let Some(disk) = disk {
            usage = usage
                .push(
                    progress_bar(0.0..=1.0, disk.fraction())
                        .length(120)
                        .girth(4)
                        .style(style::progress),
                )
                .push(
                    text(format!(
                        "{} of {} used",
                        format::bytes(disk.used()),
                        format::bytes(disk.total)
                    ))
                    .size(13)
                    .style(style::muted),
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

        vec![
            container(column![
                row![list, rule::vertical(1).style(style::separator), breakdown].height(Fill),
                ui::separator(),
                container(footer).padding([12, 20]),
            ])
            .height(Fill)
            .clip(true)
            .style(style::card)
            .into(),
        ]
    }

    fn total(&self) -> u64 {
        self.choices
            .iter()
            .find(|choice| choice.filter == self.filter)
            .map_or(self.current().bytes, |choice| choice.bytes)
    }

    fn slices(&self) -> Vec<Slice<'_>> {
        let mut largest: Vec<(&Vec<usize>, &Node)> = self
            .rows
            .iter()
            .filter_map(|indices| Some((indices, self.tree.node.find(indices)?)))
            .filter(|(_, node)| node.kind != Kind::Rest && node.bytes > 0)
            .collect();

        largest.sort_unstable_by_key(|(_, node)| Reverse(node.bytes));
        largest.truncate(SLICES);

        let mut slices: Vec<Slice<'_>> = largest
            .into_iter()
            .enumerate()
            .map(|(rank, (indices, node))| Slice {
                indices: Some(indices),
                name: node.name(),
                bytes: node.bytes,
                kind: Some(node.kind),
                color: donut::color(rank),
            })
            .collect();

        let shown: u64 = slices.iter().map(|slice| slice.bytes).sum();
        let rest = self.total().saturating_sub(shown);

        if rest > 0 {
            slices.push(Slice {
                indices: None,
                name: Cow::Borrowed("Everything else"),
                bytes: rest,
                kind: None,
                color: style::MUTED,
            });
        }

        slices
    }

    fn breakdown<'a>(&'a self, slices: Vec<Slice<'a>>) -> Element<'a, Message> {
        let title = match self.filter {
            Some(kind) => Cow::Borrowed(kind.title()),
            None => self.current().name(),
        };

        let legend = Column::with_children(slices.iter().map(|slice| {
            let hovered = slice.indices.is_some() && slice.indices == self.hovered.as_deref();

            let content = row![
                container(space())
                    .width(8)
                    .height(8)
                    .style(style::marker(Some(slice.color))),
                container(
                    text(ui::truncate(slice.name.clone(), LEGEND_CHARS))
                        .size(13)
                        .wrapping(Wrapping::None),
                )
                .width(Fill)
                .clip(true),
                text(format::bytes(slice.bytes))
                    .size(13)
                    .style(style::muted),
            ]
            .spacing(10)
            .align_y(Center);

            let Some(indices) = slice.indices else {
                return container(content).padding([7, 10]).into();
            };

            let mut item = button(content)
                .padding([7, 10])
                .width(Fill)
                .style(style::choice(hovered));
            if slice.kind == Some(Kind::Folder) {
                item = item.on_press(Message::Open(indices.to_vec()));
            }

            mouse_area(item)
                .on_enter(Message::Hover(Some(indices.to_vec())))
                .on_exit(Message::Unhover(indices.to_vec()))
                .into()
        }))
        .spacing(2);

        let chart = canvas(Donut {
            slices,
            total: self.total(),
            title,
            hovered: self.hovered.as_deref(),
        })
        .width(Fill)
        .height(CHART_HEIGHT);

        column![chart, scrollable(legend).height(Fill)]
            .spacing(12)
            .padding(16)
            .width(FillPortion(2))
            .height(Fill)
            .into()
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
struct Look<'a> {
    checked: bool,
    enabled: bool,
    highlighted: bool,
    color: Option<Color>,
    icon: Option<&'a FileIcon>,
}

fn entry<'a>(
    indices: &[usize],
    node: &'a Node,
    look: Look<'a>,
    location: Option<String>,
) -> Element<'a, Message> {
    let Look {
        checked,
        enabled,
        highlighted,
        color,
        icon,
    } = look;

    let icon = match (node.kind, icon) {
        (Kind::Rest, _) => container(ui::icon(Icon::Layers).size(16).style(style::muted))
            .center(ROW_ICON)
            .into(),
        (Kind::Folder, None) => ui::thumbnail(None, ROW_ICON),
        (_, icon) => match icon {
            Some(icon) => ui::thumbnail(Some(icon), ROW_ICON),
            None => space().width(ROW_ICON).height(ROW_ICON).into(),
        },
    };

    let marker = container(space())
        .width(3)
        .height(16)
        .style(style::marker(color));

    let toggle = (enabled && node.kind != Kind::Rest).then(|| {
        let indices = indices.to_vec();
        move |checked| Message::Toggle(indices.clone(), checked)
    });

    let caption = match (node.kind, location) {
        (Kind::Rest, _) => "Files under 1 MB grouped together".to_owned(),
        (_, Some(location)) => location,
        (Kind::Folder, None) => format!("{} items", format::count(node.items)),
        (Kind::File(kind), None) => kind.label().to_owned(),
    };

    let content = row![
        ui::check(checked, toggle),
        marker,
        icon,
        column![
            ui::name(node.name()),
            text(caption)
                .size(12)
                .style(style::muted)
                .wrapping(Wrapping::None),
        ]
        .spacing(2)
        .width(Fill)
        .clip(true),
        container(if highlighted && node.kind != Kind::Rest {
            ui::reveal_button(Message::Reveal(indices.to_vec()))
        } else {
            space().into()
        })
        .width(REVEAL_WIDTH)
        .height(ROW_ICON)
        .align_x(Horizontal::Right)
        .align_y(Center),
        container(text(format::bytes(node.bytes)).size(14))
            .width(SIZE_WIDTH)
            .align_x(Horizontal::Right),
        container(if node.kind == Kind::Folder {
            ui::icon(Icon::ChevronRight).size(14).style(style::muted)
        } else {
            text("")
        })
        .width(CHEVRON_WIDTH)
        .align_x(Horizontal::Right),
    ]
    .spacing(10)
    .align_y(Center);

    let row: Element<'a, Message> = if node.kind == Kind::Folder {
        button(content)
            .padding([10, 20])
            .width(Fill)
            .style(style::row(highlighted))
            .on_press(Message::Open(indices.to_vec()))
            .into()
    } else {
        container(content)
            .padding([10, 20])
            .style(style::highlight(highlighted))
            .into()
    };

    mouse_area(row)
        .on_enter(Message::Hover(Some(indices.to_vec())))
        .on_exit(Message::Unhover(indices.to_vec()))
        .into()
}
