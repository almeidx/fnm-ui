use iced::Task;

use crate::message::Message;
use crate::settings::TrayBehavior;
use crate::tray;

use super::Versi;
use super::platform;

impl Versi {
    pub(super) fn handle_window_close(&mut self) -> Task<Message> {
        self.save_window_geometry();
        if self.settings.tray_behavior == TrayBehavior::AlwaysRunning && tray::is_tray_active() {
            if let Some(id) = self.window_id {
                platform::set_dock_visible(false);
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            } else {
                Task::none()
            }
        } else {
            iced::exit()
        }
    }

    pub(super) fn handle_window_opened(&mut self, id: iced::window::Id) -> Task<Message> {
        self.window_id = Some(id);
        if self.pending_show {
            self.pending_show = false;
            self.pending_minimize = false;
            platform::set_dock_visible(true);
            Task::batch([
                iced::window::set_mode(id, iced::window::Mode::Windowed),
                iced::window::minimize(id, false),
                iced::window::gain_focus(id),
            ])
        } else if self.pending_minimize {
            self.pending_minimize = false;
            Task::batch([
                Task::done(Message::HideDockIcon),
                iced::window::set_mode(id, iced::window::Mode::Hidden),
            ])
        } else {
            Task::none()
        }
    }

    pub(super) fn save_window_geometry(&mut self) {
        if let (Some(size), Some(pos)) = (self.window_size, self.window_position) {
            self.settings.window_geometry = Some(crate::settings::WindowGeometry {
                width: size.width,
                height: size.height,
                x: pos.x as i32,
                y: pos.y as i32,
            });
            let _ = self.settings.save();
        }
    }
}
