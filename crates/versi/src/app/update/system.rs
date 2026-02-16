use iced::Task;

use crate::message::Message;
use crate::state::AppState;

use super::super::{Versi, platform};

impl Versi {
    pub(super) fn dispatch_system(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::AnimationTick => Ok(self.handle_animation_tick()),
            Message::Tick => Ok(self.handle_tick()),
            Message::WindowEvent(
                iced::window::Event::CloseRequested | iced::window::Event::Closed,
            )
            | Message::CloseWindow => Ok(self.handle_window_close()),
            Message::WindowEvent(iced::window::Event::Resized(size)) => {
                Ok(self.handle_window_resized(size))
            }
            Message::WindowEvent(iced::window::Event::Moved(point)) => {
                Ok(self.handle_window_moved(point))
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
            Message::OpenAppUpdate => Ok(self.open_app_update_url()),
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
            Message::OpenBackendUpdate => Ok(self.open_backend_update_url()),
            Message::VersionListCursorMoved(point) => Ok(self.handle_cursor_moved(point)),
            Message::ShowContextMenu {
                version,
                is_installed,
                is_default,
            } => Ok(self.show_context_menu(version, is_installed, is_default)),
            Message::CloseContextMenu => Ok(self.close_context_menu()),
            Message::ShowKeyboardShortcuts => Ok(self.show_keyboard_shortcuts()),
            Message::OpenLink(url) => Ok(open_url_task(url)),
            Message::TrayEvent(tray_msg) => Ok(self.handle_tray_event(tray_msg)),
            other => Err(Box::new(other)),
        }
    }

    fn handle_animation_tick(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let loading = state.active_environment().loading;
            state.refresh_rotation += std::f32::consts::TAU / 40.0;
            if !loading && state.refresh_rotation >= std::f32::consts::TAU {
                state.refresh_rotation = 0.0;
            }
        }
        Task::none()
    }

    fn handle_tick(&mut self) -> Task<Message> {
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
        Task::none()
    }

    fn handle_window_resized(&mut self, size: iced::Size) -> Task<Message> {
        self.window_size = Some(size);
        Task::none()
    }

    fn handle_window_moved(&mut self, point: iced::Point) -> Task<Message> {
        self.window_position = Some(point);
        Task::none()
    }

    fn open_app_update_url(&self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && let Some(update) = &state.app_update
        {
            return open_url_task(update.release_url.clone());
        }
        Task::none()
    }

    fn open_backend_update_url(&self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && let Some(update) = &state.backend_update
        {
            return open_url_task(update.release_url.clone());
        }
        Task::none()
    }

    fn handle_cursor_moved(&mut self, point: iced::Point) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.cursor_position = point;
        }
        Task::none()
    }

    fn show_context_menu(
        &mut self,
        version: String,
        is_installed: bool,
        is_default: bool,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.context_menu = Some(crate::state::ContextMenu {
                version,
                is_installed,
                is_default,
                position: state.cursor_position,
            });
        }
        Task::none()
    }

    fn close_context_menu(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.context_menu = None;
        }
        Task::none()
    }

    fn show_keyboard_shortcuts(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = Some(crate::state::Modal::KeyboardShortcuts);
        }
        Task::none()
    }
}

fn open_url_task(url: String) -> Task<Message> {
    Task::perform(
        async move {
            let _ = open::that(&url);
        },
        |()| Message::NoOp,
    )
}
