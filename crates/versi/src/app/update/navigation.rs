use iced::Task;

use crate::message::Message;
use crate::state::{AppState, MainViewKind};

use super::super::Versi;

impl Versi {
    pub(super) fn dispatch_navigation(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::Initialized(result) => Ok(self.handle_initialized(*result)),
            Message::EnvironmentLoaded {
                env_id,
                request_seq,
                result,
            } => Ok(self.handle_environment_loaded(&env_id, request_seq, result)),
            Message::RefreshEnvironment => Ok(self.handle_refresh_environment()),
            Message::FocusSearch => Ok(self.focus_search()),
            other => self.dispatch_navigation_selection(other),
        }
    }

    fn dispatch_navigation_selection(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::SelectPreviousVersion => {
                self.move_version_selection(false);
                Ok(Task::none())
            }
            Message::SelectNextVersion => {
                self.move_version_selection(true);
                Ok(Task::none())
            }
            Message::ActivateSelectedVersion => Ok(self.activate_hovered_version()),
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
            other => self.dispatch_navigation_data(other),
        }
    }

    fn dispatch_navigation_data(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::FetchRemoteVersions => Ok(self.handle_fetch_remote_versions()),
            Message::RemoteVersionsFetched {
                request_seq,
                result,
            } => {
                self.handle_remote_versions_fetched(request_seq, result);
                Ok(Task::none())
            }
            Message::ReleaseScheduleFetched {
                request_seq,
                result,
            } => {
                self.handle_release_schedule_fetched(request_seq, *result);
                Ok(Task::none())
            }
            Message::VersionMetadataFetched {
                request_seq,
                result,
            } => {
                self.handle_version_metadata_fetched(request_seq, *result);
                Ok(Task::none())
            }
            Message::ShowVersionDetail(version) => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::VersionDetail { version });
                }
                Ok(Task::none())
            }
            Message::CloseModal => {
                self.close_modal_or_return_to_versions();
                Ok(Task::none())
            }
            Message::OpenChangelog(version) => Ok(open_url_task(format!(
                "https://nodejs.org/en/blog/release/{version}"
            ))),
            Message::EnvironmentSelected(idx) => Ok(self.handle_environment_selected(idx)),
            Message::SelectNextEnvironment => Ok(self.select_environment_by_step(true)),
            Message::SelectPreviousEnvironment => Ok(self.select_environment_by_step(false)),
            Message::FetchReleaseSchedule => Ok(self.handle_fetch_release_schedule()),
            other => Err(Box::new(other)),
        }
    }

    fn focus_search(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.view = MainViewKind::Versions;
        }
        iced::widget::operation::focus(iced::widget::Id::new(
            crate::views::main_view::search::SEARCH_INPUT_ID,
        ))
    }

    fn move_version_selection(&mut self, next: bool) {
        if let AppState::Main(state) = &mut self.state
            && state.view == MainViewKind::Versions
            && state.modal.is_none()
        {
            let versions = state.navigable_versions(self.settings.search_results_limit);
            if versions.is_empty() {
                return;
            }

            let new_idx = match &state.hovered_version {
                Some(current) => versions.iter().position(|v| v == current).map_or(0, |i| {
                    if next {
                        (i + 1).min(versions.len() - 1)
                    } else {
                        i.saturating_sub(1)
                    }
                }),
                None => {
                    if next {
                        0
                    } else {
                        versions.len() - 1
                    }
                }
            };
            state.hovered_version = Some(versions[new_idx].clone());
        }
    }

    fn activate_hovered_version(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && state.view == MainViewKind::Versions
            && state.modal.is_none()
            && let Some(version) = state.hovered_version.clone()
        {
            if state.is_version_installed(&version) {
                return self.update(Message::SetDefault(version));
            }
            return self.update(Message::StartInstall(version));
        }
        Task::none()
    }

    fn select_environment_by_step(&mut self, next: bool) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && state.environments.len() > 1
        {
            let target = if next {
                (state.active_environment_idx + 1) % state.environments.len()
            } else if state.active_environment_idx == 0 {
                state.environments.len() - 1
            } else {
                state.active_environment_idx - 1
            };
            return self.handle_environment_selected(target);
        }
        Task::none()
    }

    fn close_modal_or_return_to_versions(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            if state.modal.is_some() {
                state.modal = None;
            } else if state.view == MainViewKind::About || state.view == MainViewKind::Settings {
                state.view = MainViewKind::Versions;
            }
        }
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
