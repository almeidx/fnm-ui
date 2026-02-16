use iced::Task;

use crate::message::Message;
use crate::state::AppState;

use super::super::{Versi, platform};

impl Versi {
    #[allow(clippy::too_many_lines)]
    pub(super) fn dispatch_system(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::AnimationTick => {
                if let AppState::Main(state) = &mut self.state {
                    let loading = state.active_environment().loading;
                    state.refresh_rotation += std::f32::consts::TAU / 40.0;
                    if !loading && state.refresh_rotation >= std::f32::consts::TAU {
                        state.refresh_rotation = 0.0;
                    }
                }
                Ok(Task::none())
            }
            Message::Tick => {
                #[cfg(target_os = "linux")]
                {
                    if crate::tray::is_tray_active() {
                        while gtk::events_pending() {
                            gtk::main_iteration();
                        }
                    }
                }
                if let AppState::Main(state) = &mut self.state {
                    let timeout = self.settings.toast_timeout_secs;
                    state.toasts.retain(|t| !t.is_expired(timeout));
                }
                Ok(Task::none())
            }
            Message::WindowEvent(
                iced::window::Event::CloseRequested | iced::window::Event::Closed,
            )
            | Message::CloseWindow => Ok(self.handle_window_close()),
            Message::WindowEvent(iced::window::Event::Resized(size)) => {
                self.window_size = Some(size);
                Ok(Task::none())
            }
            Message::WindowEvent(iced::window::Event::Moved(point)) => {
                self.window_position = Some(point);
                Ok(Task::none())
            }
            Message::WindowOpened(id) => Ok(self.handle_window_opened(id)),
            Message::HideDockIcon => {
                platform::set_dock_visible(false);
                Ok(Task::none())
            }
            Message::WindowEvent(_) => Ok(Task::none()),
            Message::AppUpdateChecked(result) => {
                self.handle_app_update_checked(*result);
                Ok(Task::none())
            }
            Message::OpenAppUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.app_update
                {
                    let url = update.release_url.clone();
                    return Ok(Task::perform(
                        async move {
                            let _ = open::that(&url);
                        },
                        |()| Message::NoOp,
                    ));
                }
                Ok(Task::none())
            }
            Message::StartAppUpdate => Ok(self.handle_start_app_update()),
            Message::AppUpdateProgress { downloaded, total } => {
                self.handle_app_update_progress(downloaded, total);
                Ok(Task::none())
            }
            Message::AppUpdateExtracting => {
                self.handle_app_update_extracting();
                Ok(Task::none())
            }
            Message::AppUpdateApplying => {
                self.handle_app_update_applying();
                Ok(Task::none())
            }
            Message::AppUpdateComplete(result) => Ok(self.handle_app_update_complete(*result)),
            Message::RestartApp => Ok(self.handle_restart_app()),
            Message::BackendUpdateChecked(result) => {
                self.handle_backend_update_checked(*result);
                Ok(Task::none())
            }
            Message::OpenBackendUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.backend_update
                {
                    let url = update.release_url.clone();
                    return Ok(Task::perform(
                        async move {
                            let _ = open::that(&url);
                        },
                        |()| Message::NoOp,
                    ));
                }
                Ok(Task::none())
            }
            Message::VersionListCursorMoved(point) => {
                if let AppState::Main(state) = &mut self.state {
                    state.cursor_position = point;
                }
                Ok(Task::none())
            }
            Message::ShowContextMenu {
                version,
                is_installed,
                is_default,
            } => {
                if let AppState::Main(state) = &mut self.state {
                    state.context_menu = Some(crate::state::ContextMenu {
                        version,
                        is_installed,
                        is_default,
                        position: state.cursor_position,
                    });
                }
                Ok(Task::none())
            }
            Message::CloseContextMenu => {
                if let AppState::Main(state) = &mut self.state {
                    state.context_menu = None;
                }
                Ok(Task::none())
            }
            Message::ShowKeyboardShortcuts => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::KeyboardShortcuts);
                }
                Ok(Task::none())
            }
            Message::OpenLink(url) => Ok(Task::perform(
                async move {
                    let _ = open::that(&url);
                },
                |()| Message::NoOp,
            )),
            Message::TrayEvent(tray_msg) => Ok(self.handle_tray_event(tray_msg)),
            other => Err(Box::new(other)),
        }
    }
}
