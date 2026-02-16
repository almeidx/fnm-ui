use iced::Task;

use crate::message::Message;

use super::super::Versi;

impl Versi {
    pub(super) fn dispatch_operations(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::StartInstall(version) => Ok(self.handle_start_install(version)),
            Message::InstallComplete {
                version,
                success,
                error,
            } => Ok(self.handle_install_complete(&version, success, error)),
            Message::RequestUninstall(version) => Ok(self.handle_uninstall(version)),
            Message::ConfirmUninstallDefault(version) => {
                Ok(self.handle_confirm_uninstall_default(version))
            }
            Message::UninstallComplete {
                version,
                success,
                error,
            } => Ok(self.handle_uninstall_complete(&version, success, error)),
            Message::RequestBulkUpdateMajors => Ok(self.handle_request_bulk_update_majors()),
            Message::RequestBulkUninstallEOL => Ok(self.handle_request_bulk_uninstall_eol()),
            Message::RequestBulkUninstallMajor { major } => {
                Ok(self.handle_request_bulk_uninstall_major(major))
            }
            Message::ConfirmBulkUpdateMajors => Ok(self.handle_confirm_bulk_update_majors()),
            Message::ConfirmBulkUninstallEOL => Ok(self.handle_confirm_bulk_uninstall_eol()),
            Message::ConfirmBulkUninstallMajor { major } => {
                Ok(self.handle_confirm_bulk_uninstall_major(major))
            }
            Message::RequestBulkUninstallMajorExceptLatest { major } => {
                Ok(self.handle_request_bulk_uninstall_major_except_latest(major))
            }
            Message::ConfirmBulkUninstallMajorExceptLatest { major } => {
                Ok(self.handle_confirm_bulk_uninstall_major_except_latest(major))
            }
            Message::CancelBulkOperation => {
                self.handle_close_modal();
                Ok(Task::none())
            }
            Message::SetDefault(version) => Ok(self.handle_set_default(version)),
            Message::DefaultChanged { success, error } => {
                Ok(self.handle_default_changed(success, error))
            }
            other => Err(Box::new(other)),
        }
    }
}
