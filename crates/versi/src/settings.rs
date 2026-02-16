use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use versi_platform::AppPaths;

use crate::backend_kind::BackendKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub theme: ThemeSetting,

    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_hours: u64,

    #[serde(default)]
    pub tray_behavior: TrayBehavior,

    #[serde(default)]
    pub start_minimized: bool,

    #[serde(default)]
    pub launch_at_login: bool,

    #[serde(default)]
    pub fnm_dir: Option<PathBuf>,

    #[serde(default)]
    pub node_dist_mirror: Option<String>,

    #[serde(default)]
    #[serde(
        deserialize_with = "deserialize_backend_shell_options",
        serialize_with = "serialize_backend_shell_options"
    )]
    pub backend_shell_options: HashMap<BackendKind, ShellOptions>,

    #[serde(default, skip_serializing)]
    shell_options: Option<ShellOptions>,

    #[serde(default)]
    pub preferred_backend: Option<BackendKind>,

    #[serde(default)]
    pub debug_logging: bool,

    #[serde(default)]
    pub window_geometry: Option<WindowGeometry>,

    #[serde(default = "default_install_timeout")]
    pub install_timeout_secs: u64,

    #[serde(default = "default_operation_timeout")]
    pub uninstall_timeout_secs: u64,

    #[serde(default = "default_operation_timeout")]
    pub set_default_timeout_secs: u64,

    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout_secs: u64,

    #[serde(default = "default_http_timeout")]
    pub http_timeout_secs: u64,

    #[serde(default = "default_toast_timeout")]
    pub toast_timeout_secs: u64,

    #[serde(default = "default_max_visible_toasts")]
    pub max_visible_toasts: usize,

    #[serde(default = "default_search_results_limit")]
    pub search_results_limit: usize,

    #[serde(default = "default_modal_preview_limit")]
    pub modal_preview_limit: usize,

    #[serde(default = "default_max_log_size_bytes")]
    pub max_log_size_bytes: u64,

    #[serde(default = "default_retry_delays")]
    pub retry_delays_secs: Vec<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShellOptions {
    #[serde(default = "default_true")]
    pub use_on_cd: bool,

    #[serde(default)]
    pub resolve_engines: bool,

    #[serde(default)]
    pub corepack_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            use_on_cd: true,
            resolve_engines: false,
            corepack_enabled: false,
        }
    }
}

fn default_cache_ttl() -> u64 {
    1
}

fn default_install_timeout() -> u64 {
    600
}

fn default_operation_timeout() -> u64 {
    60
}

fn default_fetch_timeout() -> u64 {
    30
}

fn default_http_timeout() -> u64 {
    10
}

fn default_toast_timeout() -> u64 {
    5
}

fn default_max_visible_toasts() -> usize {
    3
}

fn default_search_results_limit() -> usize {
    20
}

fn default_modal_preview_limit() -> usize {
    10
}

fn default_max_log_size_bytes() -> u64 {
    5 * 1024 * 1024
}

fn default_retry_delays() -> Vec<u64> {
    vec![0, 2, 5, 15]
}

fn deserialize_backend_shell_options<'de, D>(
    deserializer: D,
) -> Result<HashMap<BackendKind, ShellOptions>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, ShellOptions>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .filter_map(|(name, options)| BackendKind::from_name(&name).map(|kind| (kind, options)))
        .collect())
}

fn serialize_backend_shell_options<S>(
    map: &HashMap<BackendKind, ShellOptions>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let raw: HashMap<&str, &ShellOptions> = map
        .iter()
        .map(|(kind, options)| (kind.as_str(), options))
        .collect();
    raw.serialize(serializer)
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeSetting::System,
            cache_ttl_hours: 1,
            tray_behavior: TrayBehavior::WhenWindowOpen,
            start_minimized: false,
            launch_at_login: false,
            fnm_dir: None,
            node_dist_mirror: None,
            preferred_backend: None,
            backend_shell_options: HashMap::new(),
            shell_options: None,
            debug_logging: false,
            window_geometry: None,
            install_timeout_secs: default_install_timeout(),
            uninstall_timeout_secs: default_operation_timeout(),
            set_default_timeout_secs: default_operation_timeout(),
            fetch_timeout_secs: default_fetch_timeout(),
            http_timeout_secs: default_http_timeout(),
            toast_timeout_secs: default_toast_timeout(),
            max_visible_toasts: default_max_visible_toasts(),
            search_results_limit: default_search_results_limit(),
            modal_preview_limit: default_modal_preview_limit(),
            max_log_size_bytes: default_max_log_size_bytes(),
            retry_delays_secs: default_retry_delays(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let Ok(paths) = AppPaths::new() else {
            return Self::default();
        };
        let settings_path = paths.settings_file();

        let mut settings: Self = if settings_path.exists() {
            match std::fs::read_to_string(&settings_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        };

        if let Some(legacy) = settings.shell_options.take()
            && settings.backend_shell_options.is_empty()
        {
            settings
                .backend_shell_options
                .insert(BackendKind::Fnm, legacy);
        }

        settings
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let paths = AppPaths::new().map_err(std::io::Error::other)?;
        paths.ensure_dirs()?;

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(paths.settings_file(), content)?;
        Ok(())
    }

    pub fn shell_options_for(&self, backend: BackendKind) -> ShellOptions {
        self.backend_shell_options
            .get(&backend)
            .copied()
            .unwrap_or_default()
    }

    pub fn shell_options_for_mut(&mut self, backend: BackendKind) -> &mut ShellOptions {
        self.backend_shell_options.entry(backend).or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

impl WindowGeometry {
    pub fn is_likely_visible(&self) -> bool {
        const MIN_VISIBLE: f32 = -50.0;
        const MAX_COORD: f32 = 16_384.0;
        const MIN_SIZE: f32 = 100.0;

        self.x > MIN_VISIBLE
            && self.y > MIN_VISIBLE
            && self.x < MAX_COORD
            && self.y < MAX_COORD
            && self.width >= MIN_SIZE
            && self.height >= MIN_SIZE
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum TrayBehavior {
    #[default]
    WhenWindowOpen,
    AlwaysRunning,
    Disabled,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AppSettings, BackendKind, ShellOptions, WindowGeometry};

    #[test]
    fn shell_options_default_enables_use_on_cd_only() {
        let options = ShellOptions::default();

        assert!(options.use_on_cd);
        assert!(!options.resolve_engines);
        assert!(!options.corepack_enabled);
    }

    #[test]
    fn app_settings_defaults_match_expected_timeouts() {
        let settings = AppSettings::default();

        assert_eq!(settings.cache_ttl_hours, 1);
        assert_eq!(settings.install_timeout_secs, 600);
        assert_eq!(settings.uninstall_timeout_secs, 60);
        assert_eq!(settings.set_default_timeout_secs, 60);
        assert_eq!(settings.fetch_timeout_secs, 30);
        assert_eq!(settings.http_timeout_secs, 10);
        assert_eq!(settings.toast_timeout_secs, 5);
        assert_eq!(settings.max_visible_toasts, 3);
        assert_eq!(settings.search_results_limit, 20);
        assert_eq!(settings.modal_preview_limit, 10);
        assert_eq!(settings.max_log_size_bytes, 5 * 1024 * 1024);
        assert_eq!(settings.retry_delays_secs, vec![0, 2, 5, 15]);
    }

    #[test]
    fn backend_shell_options_deserialization_ignores_unknown_backends() {
        let value = json!({
            "backend_shell_options": {
                "fnm": { "use_on_cd": true, "resolve_engines": true, "corepack_enabled": false },
                "nvm": { "use_on_cd": false, "resolve_engines": false, "corepack_enabled": true },
                "volta": { "use_on_cd": true, "resolve_engines": true, "corepack_enabled": true }
            }
        });

        let settings: AppSettings =
            serde_json::from_value(value).expect("settings JSON should deserialize");

        assert_eq!(settings.backend_shell_options.len(), 2);
        assert!(
            settings
                .backend_shell_options
                .contains_key(&BackendKind::Fnm)
        );
        assert!(
            settings
                .backend_shell_options
                .contains_key(&BackendKind::Nvm)
        );
    }

    #[test]
    fn backend_shell_options_serialization_uses_backend_names() {
        let mut settings = AppSettings::default();
        settings.backend_shell_options.insert(
            BackendKind::Fnm,
            ShellOptions {
                use_on_cd: false,
                resolve_engines: true,
                corepack_enabled: true,
            },
        );

        let value = serde_json::to_value(settings).expect("settings should serialize");
        let backend_options = &value["backend_shell_options"];

        assert!(backend_options.get("fnm").is_some());
        assert!(backend_options.get("nvm").is_none());
    }

    #[test]
    fn shell_options_for_returns_default_if_not_overridden() {
        let settings = AppSettings::default();

        let options = settings.shell_options_for(BackendKind::Nvm);

        assert_eq!(options.use_on_cd, ShellOptions::default().use_on_cd);
        assert_eq!(
            options.resolve_engines,
            ShellOptions::default().resolve_engines
        );
        assert_eq!(
            options.corepack_enabled,
            ShellOptions::default().corepack_enabled
        );
    }

    #[test]
    fn shell_options_for_mut_inserts_default_entry() {
        let mut settings = AppSettings::default();

        settings
            .shell_options_for_mut(BackendKind::Fnm)
            .resolve_engines = true;

        let stored = settings
            .backend_shell_options
            .get(&BackendKind::Fnm)
            .expect("mutable accessor should insert missing backend options");
        assert!(stored.resolve_engines);
        assert!(stored.use_on_cd);
    }

    #[test]
    fn window_geometry_visibility_checks_bounds() {
        let visible = WindowGeometry {
            width: 900.0,
            height: 600.0,
            x: 200.0,
            y: 100.0,
        };
        assert!(visible.is_likely_visible());

        let too_small = WindowGeometry {
            width: 90.0,
            height: 99.0,
            x: 0.0,
            y: 0.0,
        };
        assert!(!too_small.is_likely_visible());

        let out_of_bounds = WindowGeometry {
            width: 900.0,
            height: 600.0,
            x: 20_000.0,
            y: 100.0,
        };
        assert!(!out_of_bounds.is_likely_visible());
    }
}
