use std::cell::RefCell;

use iced::Subscription;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::message::Message;
use crate::settings::TrayBehavior;
use crate::state::EnvironmentState;

thread_local! {
    static TRAY_ICON: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone)]
pub enum TrayMessage {
    ShowWindow,
    HideWindow,
    OpenSettings,
    OpenAbout,
    Quit,
    SetDefault { env_index: usize, version: String },
}

pub struct TrayMenuData {
    pub environments: Vec<EnvironmentData>,
    pub window_visible: bool,
}

pub struct EnvironmentData {
    pub name: String,
    pub env_index: usize,
    pub versions: Vec<VersionData>,
}

pub struct VersionData {
    pub version: String,
    pub is_default: bool,
}

impl TrayMenuData {
    pub fn from_environments(environments: &[EnvironmentState], window_visible: bool) -> Self {
        Self {
            window_visible,
            environments: environments
                .iter()
                .enumerate()
                .filter(|(_, env)| env.available && !env.installed_versions.is_empty())
                .map(|(idx, env)| EnvironmentData {
                    name: env.name.clone(),
                    env_index: idx,
                    versions: env
                        .installed_versions
                        .iter()
                        .map(|v| VersionData {
                            version: v.version.to_string(),
                            is_default: v.is_default,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub fn init_tray(behavior: &TrayBehavior) -> Result<(), Box<dyn std::error::Error>> {
    if *behavior == TrayBehavior::Disabled {
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    if !has_tray_host() {
        return Err("no tray host detected (StatusNotifierWatcher not registered on D-Bus)".into());
    }

    let icon = load_icon()?;
    let menu = build_menu(&TrayMenuData {
        environments: vec![],
        window_visible: true,
    });

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Versi")
        .with_icon(icon)
        .build()?;

    TRAY_ICON.with(|cell| {
        *cell.borrow_mut() = Some(tray_icon);
    });

    Ok(())
}

#[cfg(target_os = "linux")]
fn has_tray_host() -> bool {
    std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus.NameHasOwner",
            "string:org.kde.StatusNotifierWatcher",
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("true"))
        .unwrap_or(false)
}

pub fn destroy_tray() {
    TRAY_ICON.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub fn is_tray_active() -> bool {
    TRAY_ICON.with(|cell| cell.borrow().is_some())
}

fn load_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let icon_bytes = include_bytes!("../../../assets/logo.png");
    let img = image::load_from_memory(icon_bytes)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).map_err(Into::into)
}

fn build_menu(data: &TrayMenuData) -> Menu {
    let menu = Menu::new();
    let show_multiple_envs = data.environments.len() > 1;

    for (i, env) in data.environments.iter().enumerate() {
        if show_multiple_envs {
            let _ = menu.append(&MenuItem::with_id(
                MenuId::new(format!("env_header:{}", env.env_index)),
                &env.name,
                false,
                None,
            ));
        }

        for ver in &env.versions {
            let label = if ver.is_default {
                format!("{} ✓", ver.version)
            } else {
                ver.version.clone()
            };

            let _ = menu.append(&MenuItem::with_id(
                MenuId::new(format!("set:{}:{}", env.env_index, ver.version)),
                label,
                true,
                None,
            ));
        }

        if show_multiple_envs && i < data.environments.len() - 1 {
            let _ = menu.append(&PredefinedMenuItem::separator());
        }
    }

    if !data.environments.is_empty() && data.environments.iter().any(|e| !e.versions.is_empty()) {
        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    if data.window_visible {
        let _ = menu.append(&MenuItem::with_id(
            MenuId::new("hide_window"),
            "Hide Versi",
            true,
            None,
        ));
    } else {
        let _ = menu.append(&MenuItem::with_id(
            MenuId::new("show_window"),
            "Open Versi",
            true,
            None,
        ));
    }
    let _ = menu.append(&MenuItem::with_id(
        MenuId::new("open_settings"),
        "Settings",
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        MenuId::new("open_about"),
        "About",
        true,
        None,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(MenuId::new("quit"), "Quit", true, None));

    menu
}

pub fn update_menu(data: &TrayMenuData) {
    TRAY_ICON.with(|cell| {
        if let Some(tray) = cell.borrow().as_ref() {
            let menu = build_menu(data);
            tray.set_menu(Some(Box::new(menu)));
        }
    });
}

fn parse_menu_event(id: &str) -> Option<TrayMessage> {
    match id {
        "show_window" => Some(TrayMessage::ShowWindow),
        "hide_window" => Some(TrayMessage::HideWindow),
        "open_settings" => Some(TrayMessage::OpenSettings),
        "open_about" => Some(TrayMessage::OpenAbout),
        "quit" => Some(TrayMessage::Quit),
        s if s.starts_with("set:") => {
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            if parts.len() == 3 {
                let env_index = parts[1].parse().ok()?;
                let version = parts[2].to_string();
                Some(TrayMessage::SetDefault { env_index, version })
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn tray_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        iced::futures::stream::unfold((), |()| async {
            let receiver = MenuEvent::receiver();

            loop {
                if let Ok(event) = receiver.try_recv() {
                    let id_str = event.id().as_ref();
                    if let Some(msg) = parse_menu_event(id_str) {
                        return Some((Message::TrayEvent(msg), ()));
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
    })
}
