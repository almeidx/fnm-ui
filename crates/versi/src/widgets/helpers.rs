use iced::widget::{button, container, row, text, tooltip};
use iced::{Alignment, Element};

use crate::icon;
use crate::message::Message;
use crate::state::MainViewKind;
use crate::theme::styles;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NavActiveStates {
    home: bool,
    settings: bool,
    about: bool,
}

fn nav_active_states(active_view: &MainViewKind) -> NavActiveStates {
    NavActiveStates {
        home: *active_view == MainViewKind::Versions,
        settings: *active_view == MainViewKind::Settings,
        about: *active_view == MainViewKind::About,
    }
}

fn should_spin_refresh_icon(refresh_rotation: f32) -> bool {
    refresh_rotation != 0.0
}

fn nav_button_style(is_active: bool) -> fn(&iced::Theme, button::Status) -> button::Style {
    if is_active {
        styles::ghost_button_active
    } else {
        styles::ghost_button
    }
}

pub fn styled_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
    position: tooltip::Position,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label).size(12))
            .padding([4, 8])
            .style(styles::tooltip_container),
        position,
    )
    .gap(4.0)
    .into()
}

pub fn nav_icons<'a>(active_view: &MainViewKind, refresh_rotation: f32) -> Element<'a, Message> {
    let refresh_icon = if should_spin_refresh_icon(refresh_rotation) {
        icon::refresh_spinning(16.0, refresh_rotation)
    } else {
        icon::refresh(16.0)
    };

    let active_states = nav_active_states(active_view);
    let home_style = nav_button_style(active_states.home);
    let settings_style = nav_button_style(active_states.settings);
    let about_style = nav_button_style(active_states.about);

    row![
        styled_tooltip(
            button(refresh_icon)
                .on_press(Message::RefreshEnvironment)
                .style(styles::ghost_button)
                .padding([4, 6]),
            "Refresh",
            tooltip::Position::Bottom,
        ),
        styled_tooltip(
            button(icon::home(16.0))
                .on_press(Message::NavigateToVersions)
                .style(home_style)
                .padding([4, 6]),
            "Home",
            tooltip::Position::Bottom,
        ),
        styled_tooltip(
            button(icon::settings(16.0))
                .on_press(Message::NavigateToSettings)
                .style(settings_style)
                .padding([4, 6]),
            "Settings",
            tooltip::Position::Bottom,
        ),
        styled_tooltip(
            button(icon::info(16.0))
                .on_press(Message::NavigateToAbout)
                .style(about_style)
                .padding([4, 6]),
            "About",
            tooltip::Position::Bottom,
        ),
    ]
    .spacing(2)
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{nav_active_states, should_spin_refresh_icon};
    use crate::state::MainViewKind;

    #[test]
    fn nav_active_states_marks_single_active_view() {
        let versions = nav_active_states(&MainViewKind::Versions);
        assert!(versions.home);
        assert!(!versions.settings);
        assert!(!versions.about);

        let settings = nav_active_states(&MainViewKind::Settings);
        assert!(!settings.home);
        assert!(settings.settings);
        assert!(!settings.about);

        let about = nav_active_states(&MainViewKind::About);
        assert!(!about.home);
        assert!(!about.settings);
        assert!(about.about);
    }

    #[test]
    fn refresh_icon_spin_is_based_on_rotation() {
        assert!(!should_spin_refresh_icon(0.0));
        assert!(should_spin_refresh_icon(0.1));
        assert!(should_spin_refresh_icon(-0.2));
    }
}
