use std::sync::LazyLock;

use iced::font::Weight;
use iced::gradient::Linear;
use iced::theme::Palette;
use iced::widget::{button, checkbox, container, progress_bar, text};
use iced::{Background, Border, Color, Degrees, Font, Gradient, Shadow, Theme, Vector, color};

pub const BOLD: Font = Font {
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

pub const ICONS: Font = Font::with_name("lucide");

const ACCENT: Color = color!(0xb83dba);
const FOLDER: Color = color!(0x6cc3ff);

static THEME: LazyLock<Theme> = LazyLock::new(|| {
    Theme::custom(
        "Derust",
        Palette {
            background: color!(0x24145e),
            text: Color::WHITE,
            primary: ACCENT,
            success: color!(0x5fd38d),
            warning: color!(0xf5b04c),
            danger: color!(0xff5c7a),
        },
    )
});

pub fn theme() -> Theme {
    THEME.clone()
}

fn white(alpha: f32) -> Color {
    Color::WHITE.scale_alpha(alpha)
}

pub fn window(_theme: &Theme) -> container::Style {
    let gradient = Linear::new(Degrees(135.0))
        .add_stop(0.0, color!(0x2b1872))
        .add_stop(0.55, color!(0x3a1a8c))
        .add_stop(1.0, color!(0x1b0f45));

    container::Style {
        background: Some(Background::Gradient(Gradient::Linear(gradient))),
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::BLACK.scale_alpha(0.12).into()),
        ..container::Style::default()
    }
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(white(0.07).into()),
        border: Border {
            radius: 16.0.into(),
            width: 1.0,
            color: white(0.1),
        },
        ..container::Style::default()
    }
}

pub fn tooltip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(color!(0x1b0f45).into()),
        text_color: Some(Color::WHITE),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: white(0.12),
        },
        ..container::Style::default()
    }
}

pub fn muted(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(white(0.6)),
    }
}

pub fn subtle(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(white(0.8)),
    }
}

pub fn folder(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(FOLDER),
    }
}

pub fn notice(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().success),
    }
}

pub fn progress(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: white(0.12).into(),
        bar: ACCENT.into(),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
    }
}

pub fn nav(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if active {
            Some(white(0.14))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(white(0.07))
        } else {
            None
        };

        button::Style {
            background: background.map(Background::Color),
            text_color: if active { Color::WHITE } else { white(0.75) },
            border: Border {
                radius: 14.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

pub fn remove(_theme: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (ACCENT, Color::WHITE),
        button::Status::Hovered => (color!(0xc94bcb), Color::WHITE),
        button::Status::Pressed => (color!(0xa23aa4), Color::WHITE),
        button::Status::Disabled => (ACCENT.scale_alpha(0.35), white(0.45)),
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border {
            radius: 22.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.16,
        button::Status::Pressed => 0.2,
        _ => 0.1,
    };

    button::Style {
        background: Some(white(alpha).into()),
        text_color: Color::WHITE,
        border: Border {
            radius: 22.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn scan(_theme: &Theme, status: button::Status) -> button::Style {
    let (alpha, glow) = match status {
        button::Status::Active => (0.12, 0.45),
        button::Status::Hovered => (0.18, 0.6),
        button::Status::Pressed => (0.22, 0.6),
        button::Status::Disabled => (0.06, 0.0),
    };

    button::Style {
        background: Some(white(alpha).into()),
        text_color: if matches!(status, button::Status::Disabled) {
            white(0.4)
        } else {
            Color::WHITE
        },
        border: Border {
            radius: 80.0.into(),
            width: 2.0,
            color: white(0.3),
        },
        shadow: Shadow {
            color: ACCENT.scale_alpha(glow),
            offset: Vector::ZERO,
            blur_radius: 48.0,
        },
        ..button::Style::default()
    }
}

pub fn check(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (checked, hovered) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true),
        checkbox::Status::Disabled { is_checked } => (is_checked, false),
    };

    let background = match (checked, hovered) {
        (true, _) => ACCENT,
        (false, true) => white(0.12),
        (false, false) => white(0.06),
    };

    checkbox::Style {
        background: background.into(),
        icon_color: Color::WHITE,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: if checked { ACCENT } else { white(0.35) },
        },
        text_color: None,
    }
}
