//! Settings import/export flows.
//!
//! Handles messages: ExportSettings, SettingsExported, ImportSettings, SettingsImported

use iced::Task;

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::Versi;

const SETTINGS_DIALOG_CANCELLED: &str = "Cancelled";

impl Versi {
    pub(super) fn handle_export_settings(&self) -> Task<Message> {
        let settings = self.settings.clone();
        Task::perform(
            async move {
                let dialog = rfd::AsyncFileDialog::new()
                    .set_file_name("versi-settings.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                    .await;
                match dialog {
                    Some(handle) => {
                        let content = serde_json::to_string_pretty(&settings)
                            .map_err(|e| AppError::message(e.to_string()))?;
                        let path = handle.path().to_path_buf();
                        tokio::fs::write(&path, content)
                            .await
                            .map_err(|e| AppError::message(e.to_string()))?;
                        Ok(path)
                    }
                    None => Err(AppError::from(SETTINGS_DIALOG_CANCELLED)),
                }
            },
            Message::SettingsExported,
        )
    }

    pub(super) fn handle_settings_exported(
        &mut self,
        result: Result<std::path::PathBuf, AppError>,
    ) -> Task<Message> {
        if let Err(e) = result
            && !is_settings_dialog_cancelled(&e)
            && let AppState::Main(state) = &mut self.state
        {
            let id = state.next_toast_id();
            state.add_toast(crate::state::Toast::error(
                id,
                format!("Export failed: {}", e),
            ));
        }
        Task::none()
    }

    pub(super) fn handle_import_settings(&self) -> Task<Message> {
        Task::perform(
            async {
                let dialog = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                    .await;
                match dialog {
                    Some(handle) => {
                        let content = tokio::fs::read_to_string(handle.path())
                            .await
                            .map_err(|e| AppError::message(e.to_string()))?;
                        let imported: crate::settings::AppSettings = serde_json::from_str(&content)
                            .map_err(|e| AppError::message(e.to_string()))?;
                        imported
                            .save()
                            .map_err(|e| AppError::message(e.to_string()))?;
                        Ok(())
                    }
                    None => Err(AppError::from(SETTINGS_DIALOG_CANCELLED)),
                }
            },
            Message::SettingsImported,
        )
    }

    pub(super) fn handle_settings_imported(
        &mut self,
        result: Result<(), AppError>,
    ) -> Task<Message> {
        match result {
            Ok(()) => {
                self.settings = crate::settings::AppSettings::load();
            }
            Err(e) if !is_settings_dialog_cancelled(&e) => {
                if let AppState::Main(state) = &mut self.state {
                    let id = state.next_toast_id();
                    state.add_toast(crate::state::Toast::error(
                        id,
                        format!("Import failed: {}", e),
                    ));
                }
            }
            _ => {}
        }
        Task::none()
    }
}

fn is_settings_dialog_cancelled(error: &AppError) -> bool {
    matches!(error, AppError::Message(message) if message == SETTINGS_DIALOG_CANCELLED)
}
