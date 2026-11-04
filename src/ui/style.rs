use std::sync::LazyLock;

use iced::font::Weight;
use iced::theme::Palette;
use iced::widget::{button, checkbox, container, progress_bar, rule, text};
use iced::{Background, Border, Color, Font, Theme, color};

pub const FONT: Font = Font::with_name("Space Grotesk");

pub const MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..FONT
};

pub const SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..FONT
};

pub const ICONS: Font = Font::with_name("lucide");

pub const FONTS: [&[u8]; 4] = [
    include_bytes!("../../assets/fonts/SpaceGrotesk-Regular.ttf"),
    include_bytes!("../../assets/fonts/SpaceGrotesk-Medium.ttf"),
    include_bytes!("../../assets/fonts/SpaceGrotesk-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/SpaceGrotesk-Bold.ttf"),
];

const BACKGROUND: Color = color!(0x121212);
const SURFACE: Color = color!(0x171717);
const RAISED: Color = color!(0x1c1c1c);
const TRACK: Color = color!(0x242424);
const CONTROL: Color = color!(0x4a4a4a);
const TEXT: Color = color!(0xe3e3e3);
const STRONG: Color = color!(0xf2f2f2);
const MUTED: Color = color!(0x9c9c9c);
const ACCENT: Color = color!(0xff8d00);
const GREEN: Color = color!(0x5ad694);
const BLUE: Color = color!(0x68befc);
const YELLOW: Color = color!(0xf7c352);
const RED: Color = color!(0xf58d7a);

fn border() -> Color {
    Color::WHITE.scale_alpha(0.08)
}

static THEME: LazyLock<Theme> = LazyLock::new(|| {
    Theme::custom(
        "Derust",
        Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: ACCENT,
            success: GREEN,
            warning: YELLOW,
            danger: RED,
        },
    )
});

pub fn theme() -> Theme {
    THEME.clone()
}

#[derive(Debug, Clone, Copy)]
pub enum Tone {
    Neutral,
    Orange,
    Blue,
    Green,
}

impl Tone {
    fn color(self) -> Color {
        match self {
            Tone::Neutral => MUTED,
            Tone::Orange => ACCENT,
            Tone::Blue => BLUE,
            Tone::Green => GREEN,
        }
    }
}

pub fn window(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(BACKGROUND.into()),
        text_color: Some(TEXT),
        ..container::Style::default()
    }
}

pub fn card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: border(),
        },
        ..container::Style::default()
    }
}

pub fn table_head(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(RAISED.into()),
        ..container::Style::default()
    }
}

pub fn segment(tone: Option<Tone>) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(tone.map_or(TRACK, Tone::color).into()),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn separator(_theme: &Theme) -> rule::Style {
    rule::Style {
        color: border(),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

pub fn strong(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(STRONG),
    }
}

pub fn muted(_theme: &Theme) -> text::Style {
    text::Style { color: Some(MUTED) }
}

pub fn accent(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(ACCENT),
    }
}

pub fn progress(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: TRACK.into(),
        bar: ACCENT.into(),
        border: Border {
            radius: 2.0.into(),
            ..Border::default()
        },
    }
}

pub fn share(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: TRACK.into(),
        bar: MUTED.into(),
        border: Border {
            radius: 2.0.into(),
            ..Border::default()
        },
    }
}

pub fn nav(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

        let (background, text_color, border_color) = match (selected, hovered) {
            (true, _) => (Some(RAISED), ACCENT, border()),
            (false, true) => (Some(RAISED), STRONG, Color::TRANSPARENT),
            (false, false) => (None, TEXT, Color::TRANSPARENT),
        };

        button::Style {
            background: background.map(Background::Color),
            text_color,
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: border_color,
            },
            ..button::Style::default()
        }
    }
}

pub fn row(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Some(RAISED.into()),
        _ => None,
    };

    button::Style {
        background,
        text_color: TEXT,
        ..button::Style::default()
    }
}

pub fn link(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => STRONG,
        _ => MUTED,
    };

    button::Style {
        text_color,
        ..button::Style::default()
    }
}

pub fn link_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        text_color: STRONG,
        ..button::Style::default()
    }
}

pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    outlined(theme, status, ACCENT)
}

pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    outlined(theme, status, TEXT)
}

pub fn danger(theme: &Theme, status: button::Status) -> button::Style {
    outlined(theme, status, RED)
}

fn outlined(_theme: &Theme, status: button::Status, color: Color) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (SURFACE, color),
        button::Status::Hovered | button::Status::Pressed => (RAISED, color),
        button::Status::Disabled => (SURFACE, color.scale_alpha(0.35)),
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border(),
        },
        ..button::Style::default()
    }
}

pub fn check(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (checked, disabled) = match status {
        checkbox::Status::Active { is_checked } | checkbox::Status::Hovered { is_checked } => {
            (is_checked, false)
        }
        checkbox::Status::Disabled { is_checked } => (is_checked, true),
    };

    let background = if checked { ACCENT } else { SURFACE };
    let alpha = if disabled { 0.45 } else { 1.0 };

    checkbox::Style {
        background: background.scale_alpha(alpha).into(),
        icon_color: BACKGROUND,
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if checked { ACCENT } else { CONTROL }.scale_alpha(alpha),
        },
        text_color: None,
    }
}
