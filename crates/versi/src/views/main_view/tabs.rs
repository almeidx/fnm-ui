use iced::Element;
use iced::widget::{button, row, text};

use crate::message::Message;
use crate::state::MainState;
use crate::theme::styles;

pub fn environment_tabs_view<'a>(state: &'a MainState) -> Option<Element<'a, Message>> {
    if state.environments.len() <= 1 {
        return None;
    }

    let tabs: Vec<_> = state
        .environments
        .iter()
        .enumerate()
        .map(|(idx, env)| {
            let is_active = idx == state.active_environment_idx;

            if !env.available {
                let label = if let Some(reason) = &env.error {
                    format!("{} ({})", env.name, reason)
                } else {
                    format!("{} (Unavailable)", env.name)
                };
                return button(text(label).size(13))
                    .style(styles::disabled_tab_button)
                    .padding([8, 16])
                    .into();
            }

            let style = if is_active {
                styles::active_tab_button
            } else {
                styles::inactive_tab_button
            };

            button(text(&env.name).size(13))
                .on_press(Message::EnvironmentSelected(idx))
                .style(style)
                .padding([8, 16])
                .into()
        })
        .collect();

    Some(row(tabs).spacing(4).into())
}
