use chrono::{DateTime, Utc};
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::{MainState, NetworkStatus};
use crate::theme::styles;

pub(super) fn contextual_banners(state: &MainState) -> Option<Element<'_, Message>> {
    let env = state.active_environment();
    let schedule = state.available_versions.schedule.as_ref();

    let mut banners: Vec<Element<Message>> = Vec::new();

    if let Some(network_banner) = network_status_banner(state) {
        banners.push(network_banner);
    }

    if let Some(schedule_banner) = release_schedule_banner(state, schedule.is_some()) {
        banners.push(schedule_banner);
    }

    if let Some(update_banner) = available_updates_banner(state, env) {
        banners.push(update_banner);
    }

    if let Some(eol_banner) = eol_cleanup_banner(env, schedule) {
        banners.push(eol_banner);
    }

    if banners.is_empty() {
        None
    } else {
        Some(column(banners).spacing(8).into())
    }
}

fn network_status_banner(state: &MainState) -> Option<Element<'_, Message>> {
    match state.available_versions.network_status() {
        NetworkStatus::Offline => Some(simple_retry_banner(
            "Could not load available versions".to_string(),
            Message::FetchRemoteVersions,
        )),
        NetworkStatus::Fetching | NetworkStatus::Online => None,
        NetworkStatus::Stale => {
            let age_text = state
                .available_versions
                .disk_cached_at
                .map(|timestamp| format!(" (cached {})", format_relative_time(timestamp)))
                .unwrap_or_default();
            Some(simple_retry_banner(
                format!("Using cached data{age_text} \u{2014} could not refresh from network"),
                Message::FetchRemoteVersions,
            ))
        }
    }
}

fn release_schedule_banner(state: &MainState, has_schedule: bool) -> Option<Element<'_, Message>> {
    if state.available_versions.schedule_error.is_some() && !has_schedule {
        Some(simple_retry_banner(
            "Release schedule unavailable \u{2014} EOL detection may be inaccurate".to_string(),
            Message::FetchReleaseSchedule,
        ))
    } else {
        None
    }
}

fn available_updates_banner<'a>(
    state: &'a MainState,
    env: &'a crate::state::EnvironmentState,
) -> Option<Element<'a, Message>> {
    let update_count = env
        .version_groups
        .iter()
        .filter(|group| {
            let installed_latest = group.versions.iter().map(|v| &v.version).max();
            state
                .available_versions
                .latest_by_major
                .get(&group.major)
                .is_some_and(|latest| installed_latest.is_some_and(|installed| latest > installed))
        })
        .count();

    if update_count == 0 {
        return None;
    }

    let has_active_ops = !state.operation_queue.active_installs.is_empty()
        || !state.operation_queue.pending.is_empty();
    let label = format!(
        "{} major {} with updates available",
        update_count,
        if update_count == 1 {
            "version"
        } else {
            "versions"
        }
    );

    let button = button(
        row![
            text(label).size(13),
            Space::new().width(Length::Fill),
            text(if has_active_ops {
                "Updating..."
            } else {
                "Update All"
            })
            .size(13),
        ]
        .align_y(Alignment::Center),
    )
    .style(styles::banner_button_info)
    .padding([12, 16])
    .width(Length::Fill);

    Some(if has_active_ops {
        button.into()
    } else {
        button.on_press(Message::RequestBulkUpdateMajors).into()
    })
}

fn eol_cleanup_banner<'a>(
    env: &'a crate::state::EnvironmentState,
    schedule: Option<&'a versi_core::ReleaseSchedule>,
) -> Option<Element<'a, Message>> {
    let eol_count = schedule.map_or(0, |schedule| {
        env.version_groups
            .iter()
            .filter(|group| !schedule.is_active(group.major))
            .map(|group| group.versions.len())
            .sum::<usize>()
    });

    if eol_count == 0 {
        return None;
    }

    Some(
        button(
            row![
                text(format!(
                    "{} end-of-life {} installed",
                    eol_count,
                    if eol_count == 1 {
                        "version"
                    } else {
                        "versions"
                    }
                ))
                .size(13),
                Space::new().width(Length::Fill),
                text("Clean Up").size(13),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::RequestBulkUninstallEOL)
        .style(styles::banner_button_warning)
        .padding([12, 16])
        .width(Length::Fill)
        .into(),
    )
}

fn simple_retry_banner(label: String, retry_message: Message) -> Element<'static, Message> {
    button(
        row![
            text(label).size(13),
            Space::new().width(Length::Fill),
            text("Retry").size(13),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(retry_message)
    .style(styles::banner_button_warning)
    .padding([12, 16])
    .width(Length::Fill)
    .into()
}

fn format_relative_time(timestamp: DateTime<Utc>) -> String {
    let delta = Utc::now().signed_duration_since(timestamp);
    let minutes = delta.num_minutes();
    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes}m ago")
    } else {
        let hours = delta.num_hours();
        if hours < 24 {
            format!("{hours}h ago")
        } else {
            let days = delta.num_days();
            format!("{days}d ago")
        }
    }
}
