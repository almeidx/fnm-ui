use iced::Task;

use crate::message::Message;
use crate::state::{AppState, MainViewKind};

use super::super::Versi;

impl Versi {
    pub(super) fn dispatch_navigation(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::Initialized(result) => Ok(self.handle_initialized(result)),
            Message::EnvironmentLoaded { env_id, result } => {
                Ok(self.handle_environment_loaded(env_id, result))
            }
            Message::RefreshEnvironment => Ok(self.handle_refresh_environment()),
            Message::FocusSearch => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                Ok(iced::widget::operation::focus(iced::widget::Id::new(
                    crate::views::main_view::search::SEARCH_INPUT_ID,
                )))
            }
            Message::SelectPreviousVersion => {
                if let AppState::Main(state) = &mut self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                {
                    let versions = state.navigable_versions(self.settings.search_results_limit);
                    if !versions.is_empty() {
                        let new_idx = match &state.hovered_version {
                            Some(current) => versions
                                .iter()
                                .position(|v| v == current)
                                .map(|i| i.saturating_sub(1))
                                .unwrap_or(0),
                            None => versions.len() - 1,
                        };
                        state.hovered_version = Some(versions[new_idx].clone());
                    }
                }
                Ok(Task::none())
            }
            Message::SelectNextVersion => {
                if let AppState::Main(state) = &mut self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                {
                    let versions = state.navigable_versions(self.settings.search_results_limit);
                    if !versions.is_empty() {
                        let new_idx = match &state.hovered_version {
                            Some(current) => versions
                                .iter()
                                .position(|v| v == current)
                                .map(|i| (i + 1).min(versions.len() - 1))
                                .unwrap_or(0),
                            None => 0,
                        };
                        state.hovered_version = Some(versions[new_idx].clone());
                    }
                }
                Ok(Task::none())
            }
            Message::ActivateSelectedVersion => {
                if let AppState::Main(state) = &self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                    && let Some(version) = state.hovered_version.clone()
                {
                    if state.is_version_installed(&version) {
                        return Ok(self.update(Message::SetDefault(version)));
                    } else {
                        return Ok(self.update(Message::StartInstall(version)));
                    }
                }
                Ok(Task::none())
            }
            Message::VersionGroupToggled { major } => {
                self.handle_version_group_toggled(major);
                Ok(Task::none())
            }
            Message::SearchChanged(query) => {
                self.handle_search_changed(query);
                Ok(Task::none())
            }
            Message::SearchFilterToggled(filter) => {
                self.handle_search_filter_toggled(filter);
                Ok(Task::none())
            }
            Message::FetchRemoteVersions => Ok(self.handle_fetch_remote_versions()),
            Message::RemoteVersionsFetched(result) => {
                self.handle_remote_versions_fetched(result);
                Ok(Task::none())
            }
            Message::ReleaseScheduleFetched(result) => {
                self.handle_release_schedule_fetched(result);
                Ok(Task::none())
            }
            Message::VersionMetadataFetched(result) => {
                self.handle_version_metadata_fetched(result);
                Ok(Task::none())
            }
            Message::ShowVersionDetail(version) => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::VersionDetail { version });
                }
                Ok(Task::none())
            }
            Message::CloseModal => {
                if let AppState::Main(state) = &mut self.state {
                    if state.modal.is_some() {
                        state.modal = None;
                    } else if state.view == MainViewKind::About
                        || state.view == MainViewKind::Settings
                    {
                        state.view = MainViewKind::Versions;
                    }
                }
                Ok(Task::none())
            }
            Message::OpenChangelog(version) => {
                let url = format!("https://nodejs.org/en/blog/release/{}", version);
                Ok(Task::perform(
                    async move {
                        let _ = open::that(&url);
                    },
                    |_| Message::NoOp,
                ))
            }
            Message::EnvironmentSelected(idx) => Ok(self.handle_environment_selected(idx)),
            Message::SelectNextEnvironment => {
                if let AppState::Main(state) = &self.state
                    && state.environments.len() > 1
                {
                    let next = (state.active_environment_idx + 1) % state.environments.len();
                    return Ok(self.handle_environment_selected(next));
                }
                Ok(Task::none())
            }
            Message::SelectPreviousEnvironment => {
                if let AppState::Main(state) = &self.state
                    && state.environments.len() > 1
                {
                    let prev = if state.active_environment_idx == 0 {
                        state.environments.len() - 1
                    } else {
                        state.active_environment_idx - 1
                    };
                    return Ok(self.handle_environment_selected(prev));
                }
                Ok(Task::none())
            }
            Message::FetchReleaseSchedule => Ok(self.handle_fetch_release_schedule()),
            other => Err(Box::new(other)),
        }
    }
}
