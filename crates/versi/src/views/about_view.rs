use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;
use crate::widgets::helpers::nav_icons;

pub fn view(state: &MainState, has_tabs: bool) -> Element<'_, Message> {
    let header = row![
        text("About").size(14),
        Space::new().width(Length::Fill),
        nav_icons(&state.view, state.refresh_rotation),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let content = column![
        text(format!("Versi v{}", env!("CARGO_PKG_VERSION"))).size(14),
        Space::new().height(4),
        text("A native GUI for managing Node.js versions")
            .size(12)
            .color(crate::theme::tokens::TEXT_MUTED),
        Space::new().height(12),
        row![
            button(
                row![text("GitHub").size(12), icon::arrow_up_right(12.0),]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
            .on_press(Message::OpenLink(
                "https://github.com/almeidx/versi".to_string()
            ))
            .style(styles::secondary_button)
            .padding([6, 12]),
            button(
                row![text("fnm").size(12), icon::arrow_up_right(12.0),]
                    .spacing(4)
                    .align_y(Alignment::Center)
            )
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
        container(header).padding(iced::Padding::new(0.0).right(crate::theme::tokens::INSET_RIGHT)),
        Space::new().height(12),
        scrollable(
            content.padding(iced::Padding::default().right(crate::theme::tokens::INSET_RIGHT))
        )
        .height(Length::Fill),
    ]
    .spacing(0)
    .padding(super::content_padding(has_tabs))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
