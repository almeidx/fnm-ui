use std::collections::HashSet;

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::{InstalledVersion, VersionGroup};

use crate::icon;
use crate::message::Message;
use crate::state::SearchFilter;
use crate::theme::styles;

use super::VersionListContext;
use super::filter_version;
use super::item::version_item_view;

pub(super) fn version_group_view<'a>(
    group: &'a VersionGroup,
    default: Option<&'a versi_backend::NodeVersion>,
    search_query: &'a str,
    update_available: Option<String>,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
) -> Element<'a, Message> {
    let has_lts = group.versions.iter().any(|v| v.lts_codename.is_some());
    let has_default = group
        .versions
        .iter()
        .any(|v| default.is_some_and(|d| d == &v.version));
    let is_eol = ctx.schedule.is_some_and(|s| !s.is_active(group.major));

    let header_button = button(group_header_row(group, has_lts, has_default, is_eol))
        .on_press(Message::VersionGroupToggled { major: group.major })
        .style(|theme, status| {
            let mut style = iced::widget::button::text(theme, status);
            style.text_color = theme.palette().text;
            style
        })
        .padding([8, 12]);

    let header: Element<Message> = row![
        header_button,
        Space::new().width(Length::Fill),
        group_header_actions(group, update_available),
    ]
    .align_y(Alignment::Center)
    .into();

    if group.is_expanded {
        expanded_group_view(group, default, search_query, active_filters, ctx, header)
    } else {
        container(header)
            .style(styles::card_container)
            .padding(12)
            .width(Length::Fill)
            .into()
    }
}

fn group_header_row(
    group: &VersionGroup,
    has_lts: bool,
    has_default: bool,
    is_eol: bool,
) -> iced::widget::Row<'_, Message> {
    let chevron = if group.is_expanded {
        icon::chevron_down(12.0)
    } else {
        icon::chevron_right(12.0)
    };

    let mut header_row = row![
        chevron,
        text(format!("Node {}.x", group.major)).size(16),
        text(format!("({} installed)", group.versions.len())).size(12),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if has_lts {
        header_row = header_row.push(
            container(text("LTS").size(10))
                .padding([2, 6])
                .style(styles::badge_lts),
        );
    }
    if is_eol {
        header_row = header_row.push(
            container(text("End-of-Life").size(10))
                .padding([2, 6])
                .style(styles::badge_eol),
        );
    }
    if has_default && !group.is_expanded {
        header_row = header_row.push(
            container(text("default").size(10))
                .padding([2, 6])
                .style(styles::badge_default),
        );
    }
    header_row
}

fn group_header_actions(
    group: &VersionGroup,
    update_available: Option<String>,
) -> Element<'_, Message> {
    let mut actions = row![].spacing(8).align_y(Alignment::Center);

    if let Some(new_version) = update_available {
        let version_to_install = new_version.clone();
        actions = actions.push(
            button(container(text(format!("{new_version} available")).size(10)).padding([2, 6]))
                .on_press(Message::StartInstall(version_to_install))
                .style(styles::update_badge_button)
                .padding([0, 4]),
        );
    }

    if group.is_expanded && group.versions.len() > 1 {
        actions = actions.push(
            button(text("Keep Latest").size(10))
                .on_press(Message::RequestBulkUninstallMajorExceptLatest { major: group.major })
                .style(styles::ghost_button)
                .padding([4, 8]),
        );
        actions = actions.push(
            button(text("Uninstall All").size(10))
                .on_press(Message::RequestBulkUninstallMajor { major: group.major })
                .style(styles::ghost_button)
                .padding([4, 8]),
        );
    }

    actions.into()
}

fn expanded_group_view<'a>(
    group: &'a VersionGroup,
    default: Option<&'a versi_backend::NodeVersion>,
    search_query: &'a str,
    active_filters: &'a HashSet<SearchFilter>,
    ctx: &VersionListContext<'a>,
    header: Element<'a, Message>,
) -> Element<'a, Message> {
    let filtered_versions: Vec<&InstalledVersion> = group
        .versions
        .iter()
        .filter(|v| filter_version(v, search_query, active_filters, ctx.schedule))
        .collect();

    let items: Vec<Element<Message>> = filtered_versions
        .iter()
        .map(|version| version_item_view(version, default, ctx))
        .collect();

    container(
        column![
            header,
            container(column(items).spacing(2)).padding(iced::Padding {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 24.0,
            }),
        ]
        .spacing(4),
    )
    .style(styles::card_container)
    .padding(12)
    .into()
}
