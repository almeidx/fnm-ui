pub mod about_view;
pub mod loading;
pub mod main_view;
pub mod onboarding;
pub mod settings_view;

pub fn content_padding(has_tabs: bool) -> iced::Padding {
    if has_tabs {
        iced::Padding::new(24.0).right(0.0)
    } else {
        iced::Padding::new(24.0).top(12.0).right(0.0)
    }
}
