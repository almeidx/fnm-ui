//! Install, uninstall, and set-default operations with queuing.
//!
//! Handles messages: `StartInstall`, `InstallComplete`, Uninstall, `UninstallComplete`,
//! `SetDefault`, `DefaultChanged`, `CloseModal`

use std::time::Duration;

use iced::Task;

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, Modal, Operation, OperationRequest, Toast};

use super::Versi;
use super::async_helpers::run_with_timeout;

impl Versi {
    pub(super) fn handle_close_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
        }
    }

    pub(super) fn handle_start_install(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if state.operation_queue.has_active_install(&version)
                || state.operation_queue.has_pending_for_version(&version)
            {
                return Task::none();
            }

            if state.operation_queue.is_busy_for_install() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::Install { version });
                return Task::none();
            }

            return self.start_install_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_install_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.start_install(version.clone());

            let backend = state.backend.clone();
            let timeout = Duration::from_secs(self.settings.install_timeout_secs);

            return Task::perform(
                async move {
                    match run_with_timeout(
                        timeout,
                        "Installation",
                        backend.install(&version),
                        |e| AppError::message(e.to_string()),
                    )
                    .await
                    {
                        Ok(()) => (version, true, None),
                        Err(error) => (version, false, Some(error)),
                    }
                },
                |(version, success, error)| Message::InstallComplete {
                    version,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_install_complete(
        &mut self,
        version: &str,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.remove_completed_install(version);

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to install Node {}: {}",
                        version,
                        error.map_or_else(|| "unknown error".to_string(), |e| e.to_string())
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn handle_uninstall(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let is_default = state
                .active_environment()
                .default_version
                .as_ref()
                .is_some_and(|dv| dv.to_string() == version);

            if is_default {
                state.modal = Some(Modal::ConfirmUninstallDefault {
                    version: version.clone(),
                });
                return Task::none();
            }

            if state.operation_queue.is_busy_for_exclusive() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::Uninstall { version });
                return Task::none();
            }

            return self.start_uninstall_internal(&version);
        }
        Task::none()
    }

    pub(super) fn handle_confirm_uninstall_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;

            if state.operation_queue.is_busy_for_exclusive() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::Uninstall { version });
                return Task::none();
            }

            return self.start_uninstall_internal(&version);
        }
        Task::none()
    }

    pub(super) fn start_uninstall_internal(&mut self, version: &str) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let version_owned = version.to_string();
            state.operation_queue.start_exclusive(Operation::Uninstall {
                version: version_owned.clone(),
            });

            let backend = state.backend.clone();
            let version_clone = version_owned.clone();
            let timeout = Duration::from_secs(self.settings.uninstall_timeout_secs);

            return Task::perform(
                async move {
                    match run_with_timeout(
                        timeout,
                        "Uninstall",
                        backend.uninstall(&version_clone),
                        |e| AppError::message(e.to_string()),
                    )
                    .await
                    {
                        Ok(()) => (version_clone, true, None),
                        Err(error) => (version_clone, false, Some(error)),
                    }
                },
                |(version, success, error)| Message::UninstallComplete {
                    version,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_uninstall_complete(
        &mut self,
        version: &str,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.complete_exclusive();

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to uninstall Node {}: {}",
                        version,
                        error.map_or_else(|| "unknown error".to_string(), |e| e.to_string())
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn handle_set_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.operation_queue.is_busy_for_exclusive() {
                state
                    .operation_queue
                    .enqueue(OperationRequest::SetDefault { version });
                return Task::none();
            }

            return self.start_set_default_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_set_default_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state
                .operation_queue
                .start_exclusive(Operation::SetDefault {
                    version: version.clone(),
                });

            let backend = state.backend.clone();
            let timeout = Duration::from_secs(self.settings.set_default_timeout_secs);

            return Task::perform(
                async move {
                    match run_with_timeout(
                        timeout,
                        "Set default",
                        backend.set_default(&version),
                        |e| AppError::message(e.to_string()),
                    )
                    .await
                    {
                        Ok(()) => (true, None),
                        Err(error) => (false, Some(error)),
                    }
                },
                |(success, error)| Message::DefaultChanged { success, error },
            );
        }
        Task::none()
    }

    pub(super) fn handle_default_changed(
        &mut self,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.complete_exclusive();

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to set default: {}",
                        error.map_or_else(|| "unknown error".to_string(), |e| e.to_string())
                    ),
                ));
            }
        }

        let next_task = self.process_next_operation();
        let refresh_task = self.handle_refresh_environment();
        Task::batch([refresh_task, next_task])
    }

    pub(super) fn process_next_operation(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let (install_versions, exclusive_request) = state.operation_queue.drain_next();

            let mut tasks: Vec<Task<Message>> = Vec::new();
            for version in install_versions {
                tasks.push(self.start_install_internal(version));
            }
            if let Some(request) = exclusive_request {
                match request {
                    OperationRequest::Uninstall { version } => {
                        tasks.push(self.start_uninstall_internal(&version));
                    }
                    OperationRequest::SetDefault { version } => {
                        tasks.push(self.start_set_default_internal(version));
                    }
                    OperationRequest::Install { .. } => unreachable!(),
                }
            }

            if !tasks.is_empty() {
                return Task::batch(tasks);
            }
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::AppState;

    #[test]
    fn close_modal_clears_existing_modal() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.modal = Some(Modal::KeyboardShortcuts);
        }

        app.handle_close_modal();

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(state.modal.is_none());
    }

    #[test]
    fn start_install_ignores_duplicate_active_version() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.operation_queue.start_install("v20.11.0".to_string());
        }

        let _ = app.handle_start_install("v20.11.0".to_string());

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.operation_queue.active_installs.len(), 1);
        assert!(state.operation_queue.pending.is_empty());
    }

    #[test]
    fn start_install_queues_when_exclusive_operation_is_active() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state
                .operation_queue
                .start_exclusive(Operation::SetDefault {
                    version: "v20.11.0".to_string(),
                });
        }

        let _ = app.handle_start_install("v22.1.0".to_string());

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.operation_queue.pending.len(), 1);
        assert!(matches!(
            state.operation_queue.pending.front().map(|queued| &queued.request),
            Some(OperationRequest::Install { version }) if version == "v22.1.0"
        ));
    }

    #[test]
    fn uninstall_default_opens_confirmation_modal() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.active_environment_mut().default_version = Some(
                "v20.11.0"
                    .parse()
                    .expect("test default version should parse"),
            );
        }

        let _ = app.handle_uninstall("v20.11.0".to_string());

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(matches!(
            state.modal,
            Some(Modal::ConfirmUninstallDefault { ref version }) if version == "v20.11.0"
        ));
    }

    #[test]
    fn uninstall_queues_when_exclusive_queue_is_busy() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.active_environment_mut().default_version = None;
            state
                .operation_queue
                .start_exclusive(Operation::Uninstall {
                    version: "v18.0.0".to_string(),
                });
        }

        let _ = app.handle_uninstall("v20.11.0".to_string());

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(matches!(
            state.operation_queue.pending.front().map(|queued| &queued.request),
            Some(OperationRequest::Uninstall { version }) if version == "v20.11.0"
        ));
    }

    #[test]
    fn set_default_queues_when_exclusive_queue_is_busy() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state
                .operation_queue
                .start_exclusive(Operation::Uninstall {
                    version: "v18.0.0".to_string(),
                });
        }

        let _ = app.handle_set_default("v22.0.0".to_string());

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(matches!(
            state.operation_queue.pending.front().map(|queued| &queued.request),
            Some(OperationRequest::SetDefault { version }) if version == "v22.0.0"
        ));
    }
}
