use iced::widget::{container, text_input};
use iced::{Background, Border, Color, Shadow, Theme};

pub fn card_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let is_dark = palette.background.r < 0.5;

    container::Style {
        background: Some(Background::Color(crate::theme::tahoe::card_bg(is_dark))),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_LG.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color {
                a: if is_dark { 0.25 } else { 0.06 },
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 16.0,
        },
        text_color: None,
        snap: false,
    }
}

pub fn modal_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let is_dark = palette.background.r < 0.5;

    let bg = if is_dark {
        Color::from_rgb8(44, 44, 46)
    } else {
        Color::from_rgb8(255, 255, 255)
    };

    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_LG.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color {
                a: if is_dark { 0.4 } else { 0.15 },
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 24.0,
        },
        text_color: None,
        snap: false,
    }
}

pub fn search_input(theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let is_dark = palette.background.r < 0.5;

    let bg = if is_dark {
        Color::from_rgb8(44, 44, 46)
    } else {
        Color::from_rgb8(239, 239, 244)
    };

    let placeholder = Color {
        a: 0.4,
        ..palette.text
    };

    text_input::Style {
        background: Background::Color(bg),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        icon: palette.text,
        placeholder,
        value: palette.text,
        selection: Color {
            a: 0.3,
            ..palette.primary
        },
    }
}

pub fn version_row_hovered(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let is_dark = palette.background.r < 0.5;

    container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(255, 255, 255, 0.04)
        } else {
            Color::from_rgba8(0, 0, 0, 0.03)
        })),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}
