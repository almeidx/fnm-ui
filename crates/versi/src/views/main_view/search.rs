use iced::widget::{Space, button, container, text, text_input, tooltip};
use iced::{Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;

pub(super) fn search_bar_view<'a>(state: &'a MainState) -> Element<'a, Message> {
    let input = text_input(
        "Search or install versions (e.g., '22', 'lts')...",
        &state.search_query,
    )
    .on_input(Message::SearchChanged)
    .padding(14)
    .size(14)
    .style(styles::search_input);

    let clear_btn: Element<Message> = if state.search_query.is_empty() {
        Space::new().into()
    } else {
        tooltip(
            button(icon::close(14.0))
                .on_press(Message::SearchChanged(String::new()))
                .style(styles::ghost_button)
                .padding([6, 10]),
            text("Clear search").size(12),
            tooltip::Position::Left,
        )
        .into()
    };

    iced::widget::stack![
        input,
        container(clear_btn)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(0.0).right(4.0)),
    ]
    .into()
}
