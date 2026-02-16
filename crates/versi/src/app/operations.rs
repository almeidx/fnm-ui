//! Install, uninstall, and set-default operations with queuing.
//!
//! Handles messages: StartInstall, InstallComplete, Uninstall, UninstallComplete,
//! SetDefault, DefaultChanged, CloseModal

use std::time::Duration;

use iced::Task;

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, Modal, Operation, OperationRequest, Toast};

use super::Versi;

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
                    match tokio::time::timeout(timeout, backend.install(&version)).await {
                        Ok(Ok(())) => (version, true, None),
                        Ok(Err(e)) => (version, false, Some(AppError::message(e.to_string()))),
                        Err(_) => (
                            version,
                            false,
                            Some(AppError::timeout("Installation", timeout.as_secs())),
                        ),
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
        version: String,
        success: bool,
        error: Option<AppError>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.remove_completed_install(&version);

            if !success {
                let toast_id = state.next_toast_id();
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to install Node {}: {}",
                        version,
                        error
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "unknown error".to_string())
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

            return self.start_uninstall_internal(version);
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

            return self.start_uninstall_internal(version);
        }
        Task::none()
    }

    pub(super) fn start_uninstall_internal(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.operation_queue.start_exclusive(Operation::Uninstall {
                version: version.clone(),
            });

            let backend = state.backend.clone();
            let version_clone = version.clone();
            let timeout = Duration::from_secs(self.settings.uninstall_timeout_secs);

            return Task::perform(
                async move {
                    match tokio::time::timeout(timeout, backend.uninstall(&version_clone)).await {
                        Ok(Ok(())) => (version_clone, true, None),
                        Ok(Err(e)) => {
                            (version_clone, false, Some(AppError::message(e.to_string())))
                        }
                        Err(_) => (
                            version_clone,
                            false,
                            Some(AppError::timeout("Uninstall", timeout.as_secs())),
                        ),
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
        version: String,
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
                        error
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "unknown error".to_string())
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
                    match tokio::time::timeout(timeout, backend.set_default(&version)).await {
                        Ok(Ok(())) => (true, None),
                        Ok(Err(e)) => (false, Some(AppError::message(e.to_string()))),
                        Err(_) => (
                            false,
                            Some(AppError::timeout("Set default", timeout.as_secs())),
                        ),
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
                        error
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "unknown error".to_string())
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
                        tasks.push(self.start_uninstall_internal(version));
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
