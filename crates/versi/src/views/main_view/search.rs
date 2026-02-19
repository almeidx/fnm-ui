use std::collections::HashSet;

use iced::widget::{Space, button, container, row, text, text_input, tooltip};
use iced::{Element, Length};

use crate::icon;
use crate::message::Message;
use crate::state::{MainState, SearchFilter};
use crate::theme::styles;
use crate::widgets::helpers::styled_tooltip;

pub const SEARCH_INPUT_ID: &str = "search-input";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilterChipState {
    label: &'static str,
    filter: SearchFilter,
    active: bool,
}

fn should_show_clear_button(query: &str) -> bool {
    !query.is_empty()
}

const FILTER_CHIPS: &[(&str, SearchFilter)] = &[
    ("LTS", SearchFilter::Lts),
    ("Installed", SearchFilter::Installed),
    ("Not installed", SearchFilter::NotInstalled),
    ("EOL", SearchFilter::Eol),
    ("Active", SearchFilter::Active),
];

fn filter_chip_states(active_filters: &HashSet<SearchFilter>) -> Vec<FilterChipState> {
    FILTER_CHIPS
        .iter()
        .map(|&(label, filter)| FilterChipState {
            label,
            filter,
            active: active_filters.contains(&filter),
        })
        .collect()
}

pub(super) fn search_bar_view(state: &MainState) -> Element<'_, Message> {
    let input = text_input(
        "Search versions (e.g., '22', 'lts', 'lts/iron', 'latest')...",
        &state.search_query,
    )
    .id(SEARCH_INPUT_ID)
    .on_input(Message::SearchChanged)
    .padding(14)
    .size(14)
    .style(styles::search_input);

    let clear_btn: Element<Message> = if should_show_clear_button(&state.search_query) {
        styled_tooltip(
            button(icon::close(14.0))
                .on_press(Message::SearchChanged(String::new()))
                .style(styles::ghost_button)
                .padding([6, 10]),
            "Clear search",
            tooltip::Position::Left,
        )
    } else {
        Space::new().into()
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

fn chip_button(label: &str, filter: SearchFilter, active: bool) -> Element<'_, Message> {
    let style = if active {
        styles::filter_chip_active as fn(&iced::Theme, button::Status) -> button::Style
    } else {
        styles::filter_chip
    };

    button(text(label).size(12))
        .on_press(Message::SearchFilterToggled(filter))
        .style(style)
        .padding([4, 12])
        .into()
}

pub(super) fn filter_chips_view(active_filters: &HashSet<SearchFilter>) -> Element<'_, Message> {
    let chips = filter_chip_states(active_filters);
    let mut r = row![].spacing(8);
    for chip in &chips {
        r = r.push(chip_button(chip.label, chip.filter, chip.active));
    }
    r.into()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{filter_chip_states, should_show_clear_button};
    use crate::state::SearchFilter;

    #[test]
    fn clear_button_visibility_depends_on_query_content() {
        assert!(!should_show_clear_button(""));
        assert!(should_show_clear_button("v22"));
    }

    #[test]
    fn filter_chip_states_keep_stable_order_and_labels() {
        let states = filter_chip_states(&HashSet::new());

        assert_eq!(states[0].label, "LTS");
        assert_eq!(states[0].filter, SearchFilter::Lts);
        assert_eq!(states[1].label, "Installed");
        assert_eq!(states[1].filter, SearchFilter::Installed);
        assert_eq!(states[2].label, "Not installed");
        assert_eq!(states[2].filter, SearchFilter::NotInstalled);
        assert_eq!(states[3].label, "EOL");
        assert_eq!(states[3].filter, SearchFilter::Eol);
        assert_eq!(states[4].label, "Active");
        assert_eq!(states[4].filter, SearchFilter::Active);
    }

    #[test]
    fn filter_chip_states_mark_active_filters() {
        let states = filter_chip_states(&HashSet::from([
            SearchFilter::Lts,
            SearchFilter::NotInstalled,
            SearchFilter::Eol,
        ]));

        assert!(states[0].active);
        assert!(!states[1].active);
        assert!(states[2].active);
        assert!(states[3].active);
        assert!(!states[4].active);
    }
}
