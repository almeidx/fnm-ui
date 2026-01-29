use iced::widget::{Space, button, column, row, scrollable, text, tooltip};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::theme::styles;

pub fn view<'a>() -> Element<'a, Message> {
    let header = row![
        tooltip(
            button(text("\u{2190}").size(16))
                .on_press(Message::NavigateToVersions)
                .style(styles::ghost_button)
                .padding([4, 8]),
            text("Back").size(12),
            tooltip::Position::Bottom,
        ),
        text("About").size(22),
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content = column![
        text(format!("Versi v{}", env!("CARGO_PKG_VERSION"))).size(14),
        Space::new().height(4),
        text("A native GUI for fnm (Fast Node Manager)")
            .size(12)
            .color(iced::Color::from_rgb8(142, 142, 147)),
        Space::new().height(12),
        row![
            button(text("GitHub \u{2197}").size(12))
                .on_press(Message::OpenLink(
                    "https://github.com/almeidx/versi".to_string()
                ))
                .style(styles::secondary_button)
                .padding([6, 12]),
            button(text("fnm \u{2197}").size(12))
                .on_press(Message::OpenLink(
                    "https://github.com/Schniz/fnm".to_string()
                ))
                .style(styles::secondary_button)
                .padding([6, 12]),
        ]
        .spacing(8),
    ]
    .spacing(4)
    .width(Length::Fill);

    column![
        header,
        Space::new().height(24),
        scrollable(content.padding(iced::Padding::default().right(12))).height(Length::Fill),
    ]
    .spacing(0)
    .padding(32)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
