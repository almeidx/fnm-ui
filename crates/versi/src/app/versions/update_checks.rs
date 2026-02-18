use iced::Task;
use log::debug;
use versi_core::check_for_update;

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::super::Versi;

pub(super) fn handle_check_for_app_update(app: &mut Versi) -> Task<Message> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = app.http_client.clone();
    Task::perform(
        async move {
            check_for_update(&client, &current_version)
                .await
                .map_err(|error| AppError::update_check_failed("App", error))
        },
        |result| Message::AppUpdateChecked(Box::new(result)),
    )
}

pub(super) fn handle_app_update_checked(
    app: &mut Versi,
    result: Result<Option<versi_core::AppUpdate>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        match result {
            Ok(update) => state.app_update = update,
            Err(e) => debug!("App update check failed: {e}"),
        }
    }
}

pub(super) fn handle_check_for_backend_update(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &app.state
        && let Some(version) = &state.active_environment().backend_version
    {
        let version = version.clone();
        let client = app.http_client.clone();
        let provider = app.provider_for_kind(state.backend_name);
        return Task::perform(
            async move {
                provider
                    .check_for_update(&client, &version)
                    .await
                    .map_err(|error| AppError::update_check_failed("Backend", error.to_string()))
            },
            |result| Message::BackendUpdateChecked(Box::new(result)),
        );
    }
    Task::none()
}

pub(super) fn handle_backend_update_checked(
    app: &mut Versi,
    result: Result<Option<versi_backend::BackendUpdate>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        match result {
            Ok(update) => state.backend_update = update,
            Err(e) => debug!("Backend update check failed: {e}"),
        }
    }
}
