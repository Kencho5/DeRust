use std::borrow::Cow;
use std::f32::consts::{FRAC_PI_2, TAU};

use iced::alignment::Vertical;
use iced::widget::canvas::path::Arc;
use iced::widget::canvas::{self, Action, Event, Frame, Path, Stroke, Text};
use iced::widget::text::Alignment;
use iced::{Color, Point, Radians, Rectangle, Renderer, Theme, Vector, mouse};

use crate::format;
use crate::scan::explorer::Kind;
use crate::ui::explorer::Message;
use crate::ui::style;

const THICKNESS: f32 = 22.0;
const HOVER_GROWTH: f32 = 6.0;
const MARGIN: f32 = 8.0;
const GAP: f32 = 0.012;
const TITLE_CHARS: usize = 18;

#[derive(Debug)]
pub struct Slice<'a> {
    pub indices: Option<&'a [usize]>,
    pub name: Cow<'a, str>,
    pub bytes: u64,
    pub kind: Option<Kind>,
    pub color: Color,
}

#[derive(Debug)]
pub struct Donut<'a> {
    pub slices: Vec<Slice<'a>>,
    pub total: u64,
    pub title: Cow<'a, str>,
    pub hovered: Option<&'a [usize]>,
}

#[derive(Debug, Default)]
pub struct State {
    hovered: Option<usize>,
}

pub fn color(rank: usize) -> Color {
    style::PALETTE[rank % style::PALETTE.len()]
}

impl Donut<'_> {
    fn geometry(bounds: Rectangle) -> (Point, f32) {
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius =
            bounds.width.min(bounds.height) / 2.0 - MARGIN - HOVER_GROWTH - THICKNESS / 2.0;
        (center, radius.max(THICKNESS))
    }

    fn spans(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        let total = self.total.max(1) as f32;
        let mut start = 0.0;

        self.slices.iter().map(move |slice| {
            let sweep = slice.bytes as f32 / total * TAU;
            let span = (start, start + sweep);
            start += sweep;
            span
        })
    }

    fn hit(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let position = cursor.position_in(bounds)?;
        let (center, radius) = Self::geometry(bounds);

        let offset = position - center;
        let distance = offset.x.hypot(offset.y);
        if distance < radius - THICKNESS / 2.0 || distance > radius + THICKNESS / 2.0 + HOVER_GROWTH
        {
            return None;
        }

        let angle = (offset.y.atan2(offset.x) + FRAC_PI_2).rem_euclid(TAU);
        self.spans()
            .position(|(start, end)| (start..end).contains(&angle))
    }

    fn is_hovered(&self, slice: &Slice<'_>) -> bool {
        slice.indices.is_some() && slice.indices == self.hovered
    }
}

impl canvas::Program<Message> for Donut<'_> {
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
                let indices = hovered
                    .and_then(|index| self.slices[index].indices)
                    .map(<[usize]>::to_vec);
                Some(Action::publish(Message::Hover(indices)))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let slice = &self.slices[self.hit(bounds, cursor)?];
                let indices = slice.indices.filter(|_| slice.kind == Some(Kind::Folder))?;

                Some(Action::publish(Message::Open(indices.to_vec())).and_capture())
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
        let (center, radius) = Self::geometry(bounds);

        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_width(THICKNESS)
                .with_color(style::TRACK),
        );

        let highlighting = self.slices.iter().any(|slice| self.is_hovered(slice));

        for (slice, (start, end)) in self.slices.iter().zip(self.spans()) {
            let sweep = end - start;
            let gap = if self.slices.len() > 1 {
                GAP.min(sweep / 3.0)
            } else {
                0.0
            };
            if sweep - gap <= 0.0 {
                continue;
            }

            let hovered = self.is_hovered(slice);
            let (radius, width) = if hovered {
                (radius + HOVER_GROWTH / 2.0, THICKNESS + HOVER_GROWTH)
            } else {
                (radius, THICKNESS)
            };
            let alpha = if highlighting && !hovered { 0.35 } else { 1.0 };

            let arc = Path::new(|builder| {
                builder.arc(Arc {
                    center,
                    radius,
                    start_angle: Radians(start + gap / 2.0 - FRAC_PI_2),
                    end_angle: Radians(end - gap / 2.0 - FRAC_PI_2),
                });
            });

            frame.stroke(
                &arc,
                Stroke::default()
                    .with_width(width)
                    .with_color(slice.color.scale_alpha(alpha)),
            );
        }

        let focus = self.slices.iter().find(|slice| self.is_hovered(slice));
        let (value, label) = match focus {
            Some(slice) => (slice.bytes, slice.name.as_ref()),
            None => (self.total, self.title.as_ref()),
        };

        frame.fill_text(Text {
            content: format::bytes(value),
            position: center - Vector::new(0.0, 10.0),
            color: style::STRONG,
            size: 22.into(),
            font: style::MEDIUM,
            align_x: Alignment::Center,
            align_y: Vertical::Center,
            ..Text::default()
        });

        frame.fill_text(Text {
            content: fit(label, TITLE_CHARS),
            position: center + Vector::new(0.0, 14.0),
            color: style::MUTED,
            size: 12.into(),
            font: style::FONT,
            align_x: Alignment::Center,
            align_y: Vertical::Center,
            ..Text::default()
        });

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state.hovered.and_then(|index| self.slices.get(index)) {
            Some(slice) if slice.kind == Some(Kind::Folder) => mouse::Interaction::Pointer,
            _ => mouse::Interaction::default(),
        }
    }
}

fn fit(name: &str, chars: usize) -> String {
    if name.chars().count() <= chars {
        return name.to_owned();
    }

    let mut fitted: String = name.chars().take(chars.saturating_sub(1)).collect();
    fitted.push('…');
    fitted
}
