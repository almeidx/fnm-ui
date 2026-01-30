use iced::widget::container;
use iced::{Background, Border, Color, Theme};

pub fn badge_default(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        background: Some(Background::Color(Color {
            a: 0.15,
            ..palette.primary
        })),
        text_color: Some(palette.primary),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn badge_lts(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        background: Some(Background::Color(Color {
            a: 0.15,
            ..palette.success
        })),
        text_color: Some(palette.success),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn badge_eol(_theme: &Theme) -> container::Style {
    let eol_color = Color::from_rgb8(255, 149, 0);

    container::Style {
        background: Some(Background::Color(Color {
            a: 0.15,
            ..eol_color
        })),
        text_color: Some(eol_color),
        border: Border {
            radius: crate::theme::tahoe::RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}
