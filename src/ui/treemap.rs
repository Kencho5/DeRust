use std::borrow::Cow;

use iced::widget::canvas::{self, Action, Event, Frame, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::format;
use crate::scan::explorer::Kind;
use crate::ui::explorer::Message;
use crate::ui::style;

const GAP: f32 = 3.0;
const RADIUS: f32 = 6.0;
const LABEL_PADDING: f32 = 8.0;
const LABEL_HEIGHT: f32 = 44.0;
const MIN_LABEL_WIDTH: f32 = 64.0;
const MIN_LABEL_HEIGHT: f32 = 42.0;
const MIN_PREVIEW: f32 = 28.0;
const CHAR_WIDTH: f32 = 7.5;

#[derive(Debug)]
pub struct Tile<'a> {
    pub indices: &'a [usize],
    pub name: Cow<'a, str>,
    pub bytes: u64,
    pub kind: Kind,
    pub parts: Vec<u64>,
}

#[derive(Debug)]
pub struct Treemap<'a> {
    pub tiles: Vec<Tile<'a>>,
    pub hovered: Option<&'a [usize]>,
}

#[derive(Debug, Default)]
pub struct State {
    hovered: Option<usize>,
}

pub fn color(rank: usize) -> Color {
    style::PALETTE[rank % style::PALETTE.len()]
}

impl Treemap<'_> {
    fn hit(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let position = cursor.position_in(bounds)?;
        squarify(self.sizes(), bounds.size())
            .iter()
            .position(|rect| rect.contains(position))
    }

    fn sizes(&self) -> impl Iterator<Item = u64> + '_ {
        self.tiles.iter().map(|tile| tile.bytes)
    }
}

impl canvas::Program<Message> for Treemap<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) => {
                let hovered = self.hit(bounds, cursor);
                if hovered == state.hovered {
                    return None;
                }

                state.hovered = hovered;
                let indices = hovered.map(|index| self.tiles[index].indices.to_vec());
                Some(Action::publish(Message::Hover(indices)))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let tile = &self.tiles[self.hit(bounds, cursor)?];

                (tile.kind == Kind::Folder)
                    .then(|| Action::publish(Message::Open(tile.indices.to_vec())).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let rects = squarify(self.sizes(), bounds.size());

        for (rank, (tile, rect)) in self.tiles.iter().zip(rects).enumerate() {
            let size = Size::new(rect.width - GAP, rect.height - GAP);
            if size.width < 1.0 || size.height < 1.0 {
                continue;
            }

            let origin = Point::new(rect.x + GAP / 2.0, rect.y + GAP / 2.0);
            let highlighted = self.hovered == Some(tile.indices);
            let color = match tile.kind {
                Kind::Rest => style::MUTED,
                _ => color(rank),
            };

            let shape = Path::rounded_rectangle(origin, size, RADIUS.into());
            frame.fill(
                &shape,
                color.scale_alpha(if highlighted { 0.42 } else { 0.2 }),
            );
            frame.stroke(
                &shape,
                Stroke::default()
                    .with_width(1.0)
                    .with_color(color.scale_alpha(if highlighted { 0.9 } else { 0.35 })),
            );

            let labelled = size.width >= MIN_LABEL_WIDTH && size.height >= MIN_LABEL_HEIGHT;

            if labelled {
                let chars = ((size.width - LABEL_PADDING * 2.0) / CHAR_WIDTH) as usize;
                let text = Point::new(origin.x + LABEL_PADDING, origin.y + LABEL_PADDING);

                frame.fill_text(Text {
                    content: fit(&tile.name, chars),
                    position: text,
                    color: style::STRONG,
                    size: 13.into(),
                    font: style::MEDIUM,
                    ..Text::default()
                });

                frame.fill_text(Text {
                    content: format::bytes(tile.bytes),
                    position: text + Vector::new(0.0, 18.0),
                    color: style::MUTED,
                    size: 12.into(),
                    font: style::FONT,
                    ..Text::default()
                });
            }

            let preview = Size::new(
                size.width - LABEL_PADDING * 2.0,
                size.height - LABEL_HEIGHT - LABEL_PADDING,
            );

            if labelled && preview.width >= MIN_PREVIEW && preview.height >= MIN_PREVIEW {
                let offset = Vector::new(origin.x + LABEL_PADDING, origin.y + LABEL_HEIGHT);

                for part in squarify(tile.parts.iter().copied(), preview) {
                    let size = Size::new(part.width - 2.0, part.height - 2.0);
                    if size.width < 1.0 || size.height < 1.0 {
                        continue;
                    }

                    frame.fill(
                        &Path::rounded_rectangle(
                            Point::new(part.x + 1.0, part.y + 1.0) + offset,
                            size,
                            3.0.into(),
                        ),
                        color.scale_alpha(0.16),
                    );
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state.hovered.and_then(|index| self.tiles.get(index)) {
            Some(tile) if tile.kind == Kind::Folder => mouse::Interaction::Pointer,
            _ => mouse::Interaction::default(),
        }
    }
}

fn squarify(sizes: impl Iterator<Item = u64>, size: Size) -> Vec<Rectangle> {
    let sizes: Vec<u64> = sizes.collect();
    let total: u64 = sizes.iter().sum();

    if total == 0 || size.width <= 0.0 || size.height <= 0.0 {
        return vec![Rectangle::default(); sizes.len()];
    }

    let scale = size.width * size.height / total as f32;
    let areas: Vec<f32> = sizes.iter().map(|&bytes| bytes as f32 * scale).collect();

    let mut rects = Vec::with_capacity(areas.len());
    let mut free = Rectangle::new(Point::ORIGIN, size);
    let mut start = 0;

    while start < areas.len() {
        let side = free.width.min(free.height);
        let mut end = start + 1;

        while end < areas.len()
            && worst(&areas[start..=end], side) <= worst(&areas[start..end], side)
        {
            end += 1;
        }

        let row = &areas[start..end];
        let sum: f32 = row.iter().sum();

        if free.width >= free.height {
            let width = sum / free.height;
            let mut y = free.y;
            for area in row {
                let height = area / width;
                rects.push(Rectangle::new(
                    Point::new(free.x, y),
                    Size::new(width, height),
                ));
                y += height;
            }
            free.x += width;
            free.width -= width;
        } else {
            let height = sum / free.width;
            let mut x = free.x;
            for area in row {
                let width = area / height;
                rects.push(Rectangle::new(
                    Point::new(x, free.y),
                    Size::new(width, height),
                ));
                x += width;
            }
            free.y += height;
            free.height -= height;
        }

        start = end;
    }

    rects
}

fn worst(row: &[f32], side: f32) -> f32 {
    let sum: f32 = row.iter().sum();
    let largest = row.iter().copied().fold(f32::MIN, f32::max);
    let smallest = row.iter().copied().fold(f32::MAX, f32::min);
    let side = side * side;
    let sum = sum * sum;

    f32::max(side * largest / sum, sum / (side * smallest))
}

fn fit(name: &str, chars: usize) -> String {
    if name.chars().count() <= chars {
        return name.to_owned();
    }

    let mut fitted: String = name.chars().take(chars.saturating_sub(1)).collect();
    fitted.push('…');
    fitted
}
