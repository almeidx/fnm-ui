pub mod styles;

use iced::theme::Palette;
use iced::{Theme, color};

pub mod tahoe {
    pub const RADIUS_SM: f32 = 8.0;
    pub const RADIUS_MD: f32 = 12.0;
    pub const RADIUS_LG: f32 = 16.0;

    pub fn card_bg(is_dark: bool) -> iced::Color {
        if is_dark {
            iced::Color::from_rgba8(44, 44, 46, 0.72)
        } else {
            iced::Color::from_rgba8(255, 255, 255, 0.72)
        }
    }
}

pub fn light_theme() -> Theme {
    Theme::custom(
        "Versi Light".to_string(),
        Palette {
            background: color!(0x00f5_f5f7),
            text: color!(0x001d_1d1f),
            primary: color!(0x0000_7aff),
            success: color!(0x0034_c759),
            danger: color!(0x00ff_3b30),
            warning: color!(0x00ff_9500),
        },
    )
}

pub fn dark_theme() -> Theme {
    Theme::custom(
        "Versi Dark".to_string(),
        Palette {
            background: color!(0x001c_1c1e),
            text: color!(0x00f5_f5f7),
            primary: color!(0x000a_84ff),
            success: color!(0x0030_d158),
            danger: color!(0x00ff_453a),
            warning: color!(0x00ff_9f0a),
        },
    )
}
