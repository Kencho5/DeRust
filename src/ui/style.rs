use std::sync::LazyLock;

use iced::font::Weight;
use iced::overlay::menu;
use iced::theme::Palette;
use iced::widget::{button, checkbox, container, pick_list, progress_bar, rule, text};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector, color};

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

const BACKGROUND: Color = color!(0x141414);
const SURFACE: Color = color!(0x1b1b1b);
const RAISED: Color = color!(0x232323);
pub const TRACK: Color = color!(0x2c2c2c);
const CONTROL: Color = color!(0x4a4a4a);
const TEXT: Color = color!(0xe3e3e3);
pub const STRONG: Color = color!(0xf2f2f2);
pub const MUTED: Color = color!(0x9c9c9c);
pub const ACCENT: Color = color!(0xff8d00);
const GREEN: Color = color!(0x5ad694);
pub const BLUE: Color = color!(0x68befc);
const YELLOW: Color = color!(0xf7c352);

pub const PALETTE: [Color; 8] = [
    color!(0xff8d00),
    color!(0x68befc),
    color!(0xa88cf5),
    color!(0x5ad694),
    color!(0xf7c352),
    color!(0xf58db8),
    color!(0x4fd1c5),
    color!(0xf58d7a),
];
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
}

impl Tone {
    fn color(self) -> Color {
        match self {
            Tone::Neutral => MUTED,
            Tone::Orange => ACCENT,
            Tone::Blue => BLUE,
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

pub fn tip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(TRACK.into()),
        text_color: Some(STRONG),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border(),
        },
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

pub fn marker(color: Option<Color>) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: color.map(Background::Color),
        border: Border {
            radius: 2.0.into(),
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

pub fn row(highlighted: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

        button::Style {
            background: (highlighted || hovered).then(|| RAISED.into()),
            text_color: TEXT,
            ..button::Style::default()
        }
    }
}

pub fn highlight(highlighted: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: highlighted.then(|| RAISED.into()),
        ..container::Style::default()
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

pub fn dropdown(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Active => SURFACE,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => RAISED,
    };

    pick_list::Style {
        text_color: TEXT,
        placeholder_color: MUTED,
        handle_color: MUTED,
        background: background.into(),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border(),
        },
    }
}

pub fn menu(_theme: &Theme) -> menu::Style {
    menu::Style {
        background: RAISED.into(),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border(),
        },
        text_color: TEXT,
        selected_text_color: STRONG,
        selected_background: TRACK.into(),
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(0.4),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
    }
}

pub fn link_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        text_color: STRONG,
        ..button::Style::default()
    }
}

pub fn choice(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

        let (background, text_color) = match (selected, hovered) {
            (true, _) => (Some(TRACK), STRONG),
            (false, true) => (Some(RAISED), STRONG),
            (false, false) => (None, TEXT),
        };

        button::Style {
            background: background.map(Background::Color),
            text_color,
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
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
