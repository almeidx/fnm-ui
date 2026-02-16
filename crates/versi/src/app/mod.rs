mod auto_update;
mod bulk_operations;
mod environment;
mod init;
mod onboarding;
mod operations;
mod platform;
mod settings_io;
mod shell;
mod tray_handlers;
mod update;
mod versions;
mod window;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Subscription, Task, Theme};

use versi_backend::BackendProvider;

use crate::backend_kind::BackendKind;
use crate::message::Message;
use crate::settings::{AppSettings, ThemeSetting, TrayBehavior};
use crate::state::{AppState, MainViewKind};
use crate::theme::{dark_theme, light_theme};
use crate::tray;
use crate::views;

fn should_dismiss_context_menu(message: &Message) -> bool {
    !matches!(
        message,
        Message::NoOp
            | Message::Tick
            | Message::AnimationTick
            | Message::VersionListCursorMoved(_)
            | Message::VersionRowHovered(_)
            | Message::WindowEvent(_)
            | Message::SystemThemeChanged(_)
            | Message::CloseContextMenu
            | Message::ShowContextMenu { .. }
    )
}

pub struct Versi {
    pub(crate) state: AppState,
    pub(crate) settings: AppSettings,
    pub(crate) window_id: Option<iced::window::Id>,
    pub(crate) pending_minimize: bool,
    pub(crate) pending_show: bool,
    pub(crate) window_visible: bool,
    pub(crate) backend_path: PathBuf,
    pub(crate) backend_dir: Option<PathBuf>,
    pub(crate) window_size: Option<iced::Size>,
    pub(crate) window_position: Option<iced::Point>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) providers: HashMap<BackendKind, Arc<dyn BackendProvider>>,
    pub(crate) provider: Arc<dyn BackendProvider>,
    pub(crate) system_theme_mode: iced::theme::Mode,
}

impl Versi {
    pub fn new() -> (Self, Task<Message>) {
        let settings = AppSettings::load();

        let should_minimize = settings.start_minimized
            && settings.tray_behavior != TrayBehavior::Disabled
            && tray::is_tray_active();

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(settings.http_timeout_secs))
            .user_agent(format!("versi/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
        let nvm_provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());

        let mut providers: HashMap<BackendKind, Arc<dyn BackendProvider>> = HashMap::new();
        providers.insert(BackendKind::Fnm, fnm_provider.clone());
        providers.insert(BackendKind::Nvm, nvm_provider.clone());

        let preferred = settings.preferred_backend.unwrap_or(BackendKind::DEFAULT);
        let active_provider = providers.get(&preferred).cloned().unwrap_or(fnm_provider);

        let app = Self {
            state: AppState::Loading,
            settings,
            window_id: None,
            pending_minimize: should_minimize,
            pending_show: false,
            window_visible: !should_minimize,
            backend_path: PathBuf::from(active_provider.name()),
            backend_dir: None,
            window_size: None,
            window_position: None,
            http_client,
            providers: providers.clone(),
            provider: active_provider,
            system_theme_mode: iced::theme::Mode::None,
        };

        let all_providers: Vec<Arc<dyn BackendProvider>> = providers.values().cloned().collect();
        let preferred_backend = app.settings.preferred_backend.clone();
        let init_task = Task::perform(
            init::initialize(all_providers, preferred_backend),
            Message::Initialized,
        );
        let theme_task = iced::system::theme().map(Message::SystemThemeChanged);

        (app, Task::batch([init_task, theme_task]))
    }

    pub fn title(&self) -> String {
        match &self.state {
            AppState::Loading => "Versi".to_string(),
            AppState::Onboarding(_) => "Versi - Setup".to_string(),
            AppState::Main(state) => {
                if let Some(v) = &state.active_environment().default_version {
                    format!("Versi - Node {}", v)
                } else {
                    "Versi".to_string()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::Loading => views::loading::view(),
            AppState::Onboarding(state) => {
                let backend_name = state.selected_backend.unwrap_or(self.active_backend_kind());
                views::onboarding::view(state, backend_name)
            }
            AppState::Main(state) => {
                use iced::widget::{column, container};

                let tab_row = views::main_view::tabs::environment_tabs_view(state);
                let has_tabs = tab_row.is_some();

                let inner = match state.view {
                    MainViewKind::Versions => {
                        views::main_view::view(state, &self.settings, has_tabs)
                    }
                    MainViewKind::Settings => views::settings_view::view(
                        &state.settings_state,
                        &self.settings,
                        state,
                        has_tabs,
                        self.is_system_dark(),
                    ),
                    MainViewKind::About => views::about_view::view(state, has_tabs),
                };

                if let Some(tabs) = tab_row {
                    let tabs_container = container(tabs)
                        .padding(iced::Padding::new(0.0).top(12.0).left(24.0).right(24.0));
                    column![tabs_container, inner].spacing(0).into()
                } else {
                    inner
                }
            }
        }
    }

    pub fn theme(&self) -> Theme {
        match self.settings.theme {
            ThemeSetting::System => {
                if self.system_theme_mode == iced::theme::Mode::Dark {
                    dark_theme()
                } else {
                    light_theme()
                }
            }
            ThemeSetting::Light => light_theme(),
            ThemeSetting::Dark => dark_theme(),
        }
    }

    pub fn is_system_dark(&self) -> bool {
        self.system_theme_mode == iced::theme::Mode::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick_ms = {
            #[cfg(target_os = "linux")]
            {
                if tray::is_tray_active() { 100 } else { 1000 }
            }
            #[cfg(not(target_os = "linux"))]
            {
                1000u64
            }
        };
        let tick =
            iced::time::every(std::time::Duration::from_millis(tick_ms)).map(|_| Message::Tick);

        let keyboard = iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key, modifiers, ..
            }) = event
            {
                if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                    return Some(Message::CloseModal);
                }

                #[cfg(target_os = "macos")]
                let cmd = modifiers.command();
                #[cfg(not(target_os = "macos"))]
                let cmd = modifiers.control();

                if cmd && let iced::keyboard::Key::Character(c) = &key {
                    match c.as_str() {
                        "k" => return Some(Message::FocusSearch),
                        "," => return Some(Message::NavigateToSettings),
                        "r" => return Some(Message::RefreshEnvironment),
                        "w" => return Some(Message::CloseWindow),
                        _ => {}
                    }
                }

                if !cmd
                    && let iced::keyboard::Key::Character(c) = &key
                    && c.as_str() == "?"
                {
                    return Some(Message::ShowKeyboardShortcuts);
                }

                if let iced::keyboard::Key::Named(named) = &key {
                    match named {
                        iced::keyboard::key::Named::ArrowUp => {
                            return Some(Message::SelectPreviousVersion);
                        }
                        iced::keyboard::key::Named::ArrowDown => {
                            return Some(Message::SelectNextVersion);
                        }
                        iced::keyboard::key::Named::Enter => {
                            return Some(Message::ActivateSelectedVersion);
                        }
                        iced::keyboard::key::Named::Tab if cmd && modifiers.shift() => {
                            return Some(Message::SelectPreviousEnvironment);
                        }
                        iced::keyboard::key::Named::Tab if cmd => {
                            return Some(Message::SelectNextEnvironment);
                        }
                        _ => {}
                    }
                }

                None
            } else {
                None
            }
        });

        let window_events = iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Window(window_event) = event {
                Some(Message::WindowEvent(window_event))
            } else {
                None
            }
        });

        let tray_sub =
            if self.settings.tray_behavior != TrayBehavior::Disabled && tray::is_tray_active() {
                tray::tray_subscription()
            } else {
                Subscription::none()
            };

        let window_open_sub = iced::window::open_events().map(Message::WindowOpened);

        let animation_tick = if self.is_refresh_animating() {
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        };

        let theme_changes = iced::system::theme_changes().map(Message::SystemThemeChanged);

        Subscription::batch([
            tick,
            keyboard,
            window_events,
            tray_sub,
            window_open_sub,
            animation_tick,
            theme_changes,
        ])
    }

    fn is_refresh_animating(&self) -> bool {
        if let AppState::Main(state) = &self.state {
            state.refresh_rotation != 0.0
        } else {
            false
        }
    }

    fn handle_preferred_backend_changed(&mut self, name: BackendKind) -> Task<Message> {
        self.settings.preferred_backend = Some(name);
        if let Err(e) = self.settings.save() {
            log::error!("Failed to save settings: {e}");
        }

        if let AppState::Main(state) = &mut self.state {
            let is_detected = state.detected_backends.contains(&name);
            if is_detected && state.backend_name != name {
                if let Some(provider) = self.providers.get(&name) {
                    self.provider = provider.clone();
                }
                let all_providers = self.all_providers();
                let preferred = self.settings.preferred_backend.clone();
                self.state = AppState::Loading;
                return Task::perform(
                    init::initialize(all_providers, preferred),
                    Message::Initialized,
                );
            }
        }

        Task::none()
    }

    pub(crate) fn all_providers(&self) -> Vec<Arc<dyn BackendProvider>> {
        self.providers.values().cloned().collect()
    }

    pub(crate) fn provider_for_kind(&self, kind: BackendKind) -> Arc<dyn BackendProvider> {
        self.providers
            .get(&kind)
            .cloned()
            .unwrap_or_else(|| self.provider.clone())
    }

    pub(crate) fn active_provider(&self) -> Arc<dyn BackendProvider> {
        if let AppState::Main(state) = &self.state {
            self.provider_for_kind(state.backend_name)
        } else {
            self.provider.clone()
        }
    }

    pub(crate) fn active_backend_kind(&self) -> BackendKind {
        if let AppState::Main(state) = &self.state {
            state.backend_name
        } else {
            BackendKind::from_name(self.provider.name()).unwrap_or(BackendKind::DEFAULT)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use versi_backend::{BackendDetection, BackendProvider};
    use versi_platform::EnvironmentId;

    use super::{Versi, should_dismiss_context_menu};
    use crate::backend_kind::BackendKind;
    use crate::message::Message;
    use crate::settings::AppSettings;
    use crate::state::{AppState, EnvironmentState, MainState, Operation};
    use crate::tray::TrayMessage;

    fn test_app_with_two_environments() -> Versi {
        let fnm_provider: Arc<dyn BackendProvider> = Arc::new(versi_fnm::FnmProvider::new());
        let nvm_provider: Arc<dyn BackendProvider> = Arc::new(versi_nvm::NvmProvider::new());

        let mut providers: HashMap<BackendKind, Arc<dyn BackendProvider>> = HashMap::new();
        providers.insert(BackendKind::Fnm, fnm_provider.clone());
        providers.insert(BackendKind::Nvm, nvm_provider.clone());

        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("fnm")),
            version: None,
            in_path: true,
            data_dir: None,
        };
        let backend = fnm_provider.create_manager(&detection);

        let native = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        let wsl = EnvironmentState::new(
            EnvironmentId::Wsl {
                distro: "Ubuntu".to_string(),
                backend_path: "/home/user/.nvm/nvm.sh".to_string(),
            },
            BackendKind::Nvm,
            None,
        );
        let main_state =
            MainState::new_with_environments(backend, vec![native, wsl], BackendKind::Fnm);

        Versi {
            state: AppState::Main(Box::new(main_state)),
            settings: AppSettings::default(),
            window_id: None,
            pending_minimize: false,
            pending_show: false,
            window_visible: true,
            backend_path: PathBuf::from("fnm"),
            backend_dir: None,
            window_size: None,
            window_position: None,
            http_client: reqwest::Client::new(),
            providers,
            provider: fnm_provider,
            system_theme_mode: iced::theme::Mode::None,
        }
    }

    #[test]
    fn context_menu_is_dismissed_for_unrelated_messages() {
        assert!(should_dismiss_context_menu(&Message::NavigateToSettings));
        assert!(should_dismiss_context_menu(&Message::SetDefault(
            "20.10.0".to_string()
        )));
    }

    #[test]
    fn context_menu_stays_open_for_allowed_messages() {
        assert!(!should_dismiss_context_menu(&Message::Tick));
        assert!(!should_dismiss_context_menu(&Message::ShowContextMenu {
            version: "20.10.0".to_string(),
            is_installed: true,
            is_default: false,
        }));
    }

    #[test]
    fn environment_switch_updates_active_backend_kind_and_provider() {
        let mut app = test_app_with_two_environments();

        let _ = app.handle_environment_selected(1);

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.active_environment_idx, 1);
        assert_eq!(state.backend_name, BackendKind::Nvm);
        assert_eq!(app.provider.name(), BackendKind::Nvm.as_str());
    }

    #[test]
    fn tray_set_default_switches_environment_before_queueing_operation() {
        let mut app = test_app_with_two_environments();

        let _ = app.handle_tray_event(TrayMessage::SetDefault {
            env_index: 1,
            version: "20.11.0".to_string(),
        });

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.active_environment_idx, 1);
        assert_eq!(state.backend_name, BackendKind::Nvm);
        assert_eq!(app.provider.name(), BackendKind::Nvm.as_str());
        assert!(matches!(
            state.operation_queue.exclusive_op,
            Some(Operation::SetDefault { ref version }) if version == "20.11.0"
        ));
    }
}
