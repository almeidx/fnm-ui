use iced::Element;
use iced::widget::{container, text, tooltip};

use crate::message::Message;
use crate::theme::styles;

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
