use iced::widget::{button, container, text_editor};
use iced::{Background, Border, Color, Font, Shadow, Theme};

pub const BG: Color = Color::from_rgb(0.035, 0.042, 0.052);
pub const PANEL: Color = Color::from_rgb(0.055, 0.064, 0.077);
pub const RAISED: Color = Color::from_rgb(0.075, 0.085, 0.100);
pub const HOVER: Color = Color::from_rgb(0.095, 0.106, 0.122);
pub const LINE: Color = Color::from_rgb(0.135, 0.148, 0.166);
pub const TEXT: Color = Color::from_rgb(0.875, 0.890, 0.900);
pub const MUTED: Color = Color::from_rgb(0.500, 0.535, 0.575);
pub const PRIMARY: Color = Color::from_rgb(0.390, 0.735, 0.735);
pub const CANON: Color = Color::from_rgb(0.880, 0.670, 0.320);
pub const WORKING: Color = Color::from_rgb(0.480, 0.650, 0.840);
pub const UNRESOLVED: Color = Color::from_rgb(0.860, 0.750, 0.350);
pub const CONTRADICTION: Color = Color::from_rgb(0.930, 0.390, 0.340);
pub const INFERRED: Color = Color::from_rgb(0.660, 0.550, 0.840);

pub const TX02: Font = Font::new("TX-02");
pub const PHOSPHOR: Font = Font::new("Phosphor");

pub fn palette() -> iced::theme::palette::Seed {
    iced::theme::palette::Seed {
        background: BG,
        text: TEXT,
        primary: PRIMARY,
        success: PRIMARY,
        danger: CONTRADICTION,
        warning: UNRESOLVED,
    }
}

pub fn canvas(_: &Theme) -> container::Style {
    container::Style::default().background(BG)
}

pub fn panel(_: &Theme) -> container::Style {
    container::Style::default()
        .background(PANEL)
        .border(Border {
            color: LINE,
            width: 1.0,
            radius: 0.into(),
        })
}

pub fn raised(_: &Theme) -> container::Style {
    container::Style::default()
        .background(RAISED)
        .border(Border {
            color: LINE,
            width: 1.0,
            radius: 4.into(),
        })
}

pub fn inset(_: &Theme) -> container::Style {
    container::Style::default().background(BG).border(Border {
        color: LINE,
        width: 1.0,
        radius: 4.into(),
    })
}

pub fn primary_tint(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgba(PRIMARY.r, PRIMARY.g, PRIMARY.b, 0.09))
        .border(Border {
            color: Color::from_rgba(PRIMARY.r, PRIMARY.g, PRIMARY.b, 0.35),
            width: 1.0,
            radius: 3.into(),
        })
}

pub fn inferred(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgba(INFERRED.r, INFERRED.g, INFERRED.b, 0.06))
        .border(Border {
            color: Color::from_rgba(INFERRED.r, INFERRED.g, INFERRED.b, 0.50),
            width: 1.0,
            radius: 4.into(),
        })
}

pub fn unresolved(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgba(
            UNRESOLVED.r,
            UNRESOLVED.g,
            UNRESOLVED.b,
            0.05,
        ))
        .border(Border {
            color: Color::from_rgba(UNRESOLVED.r, UNRESOLVED.g, UNRESOLVED.b, 0.45),
            width: 1.0,
            radius: 4.into(),
        })
}

pub fn contradiction(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgba(
            CONTRADICTION.r,
            CONTRADICTION.g,
            CONTRADICTION.b,
            0.08,
        ))
        .border(Border {
            color: Color::from_rgba(CONTRADICTION.r, CONTRADICTION.g, CONTRADICTION.b, 0.55),
            width: 1.0,
            radius: 4.into(),
        })
}

pub fn tooltip(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb(0.065, 0.074, 0.087))
        .border(Border {
            color: LINE,
            width: 1.0,
            radius: 4.into(),
        })
        .shadow(Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: iced::Vector::new(0.0, 5.0),
            blur_radius: 18.0,
        })
}

pub fn clear_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Hovered).then_some(Background::Color(HOVER)),
        text_color: TEXT,
        border: Border {
            radius: 4.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn bordered_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(
            if matches!(status, button::Status::Hovered) {
                HOVER
            } else {
                PANEL
            },
        )),
        text_color: TEXT,
        border: Border {
            color: LINE,
            width: 1.0,
            radius: 4.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn primary_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(
            if matches!(status, button::Status::Hovered) {
                Color::from_rgb(0.46, 0.80, 0.80)
            } else {
                PRIMARY
            },
        )),
        text_color: BG,
        border: Border {
            radius: 4.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn nav_button(active: bool, _: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered);
    button::Style {
        background: (active || hovered).then_some(Background::Color(if active {
            RAISED
        } else {
            HOVER
        })),
        text_color: if active { TEXT } else { MUTED },
        border: Border {
            color: if active { LINE } else { Color::TRANSPARENT },
            width: 1.0,
            radius: 4.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn tab_button(active: bool, _: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Hovered).then_some(Background::Color(HOVER)),
        text_color: if active { TEXT } else { MUTED },
        border: Border {
            color: if active { PRIMARY } else { Color::TRANSPARENT },
            width: if active { 1.0 } else { 0.0 },
            radius: 2.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn editor(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: Background::Color(BG),
        border: Border {
            color: if focused { PRIMARY } else { LINE },
            width: 1.0,
            radius: 5.into(),
        },
        placeholder: MUTED,
        value: TEXT,
        selection: Color::from_rgba(PRIMARY.r, PRIMARY.g, PRIMARY.b, 0.25),
    }
}
