use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use versi_backend::RemoteVersion;

use crate::message::Message;
use crate::theme::styles;

use super::VersionListContext;

pub(super) fn available_version_row<'a>(
    version: &'a RemoteVersion,
    ctx: &VersionListContext<'a>,
) -> Element<'a, Message> {
    let version_str = version.version.to_string();
    let meta = ctx.metadata.and_then(|m| m.get(&version_str));
    let is_eol = ctx
        .schedule
        .map(|s| !s.is_active(version.version.major))
        .unwrap_or(false);
    let version_display = version_str.clone();
    let version_for_changelog = version_str.clone();
    let version_for_hover = version_str.clone();
    let version_for_ctx = version_str.clone();
    let is_installed = ctx.installed_set.contains(&version_str);

    let is_active = ctx.operation_queue.is_current_version(&version_str);
    let is_pending = ctx.operation_queue.has_pending_for_version(&version_str);
    let is_button_hovered = ctx
        .hovered_version
        .as_ref()
        .is_some_and(|h| h == &version_str);

    let action_button: Element<Message> = if is_active {
        button(text("Installing...").size(12))
            .style(styles::primary_button)
            .padding([6, 12])
            .into()
    } else if is_pending {
        button(text("Queued").size(12))
            .style(styles::secondary_button)
            .padding([6, 12])
            .into()
    } else if is_installed {
        let btn = if is_button_hovered {
            button(text("Uninstall").size(12))
                .on_press(Message::RequestUninstall(version_str))
                .style(styles::danger_button)
                .padding([6, 12])
        } else {
            button(text("Installed").size(12))
                .style(styles::secondary_button)
                .padding([6, 12])
        };
        mouse_area(btn)
            .on_enter(Message::VersionRowHovered(Some(version_for_hover)))
            .on_exit(Message::VersionRowHovered(None))
            .into()
    } else {
        button(text("Install").size(12))
            .on_press(Message::StartInstall(version_str))
            .style(styles::primary_button)
            .padding([6, 12])
            .into()
    };

    let mut badges = row![].spacing(6).align_y(Alignment::Center);
    if let Some(lts) = &version.lts_codename
        && !is_eol
    {
        badges = badges.push(
            container(text(format!("LTS: {}", lts)).size(11))
                .padding([2, 6])
                .style(styles::badge_lts),
        );
    }
    if is_eol {
        badges = badges.push(
            container(text("End-of-Life").size(11))
                .padding([2, 6])
                .style(styles::badge_eol),
        );
    }
    if meta.map(|m| m.security).unwrap_or(false) {
        badges = badges.push(
            container(text("Security").size(11))
                .padding([2, 6])
                .style(styles::badge_security),
        );
    }

    let date_text: Element<Message> = if let Some(date) = meta.map(|m| m.date.as_str()) {
        text(date)
            .size(11)
            .color(iced::Color::from_rgb8(142, 142, 147))
            .into()
    } else {
        Space::new().into()
    };

    mouse_area(
        row![
            button(text(version_display).size(14))
                .on_press(Message::ShowVersionDetail(version_for_changelog))
                .style(styles::ghost_button)
                .padding([2, 4])
                .width(Length::Fixed(120.0)),
            container(date_text).width(Length::Fixed(80.0)),
            badges,
            Space::new().width(Length::Fill),
            action_button,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([4, 8]),
    )
    .on_right_press(Message::ShowContextMenu {
        version: version_for_ctx,
        is_installed,
        is_default: false,
    })
    .into()
}
