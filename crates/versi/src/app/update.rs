mod navigation;
mod operations;
mod settings;
mod system;

use iced::Task;

use crate::message::Message;
use crate::state::AppState;

use super::{Versi, should_dismiss_context_menu};

type DispatchResult = Result<Task<Message>, Message>;

impl Versi {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.dismiss_context_menu_if_needed(&message);

        let message = match self.dispatch_navigation(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.dispatch_operations(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.dispatch_settings(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.dispatch_system(message) {
            Ok(task) => return task,
            Err(message) => message,
        };

        let _ = message;
        Task::none()
    }

    fn dismiss_context_menu_if_needed(&mut self, message: &Message) {
        if let AppState::Main(state) = &mut self.state
            && state.context_menu.is_some()
            && should_dismiss_context_menu(message)
        {
            state.context_menu = None;
        }
    }
}
