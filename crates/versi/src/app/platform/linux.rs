pub(crate) fn set_update_badge(visible: bool) {
    use log::debug;

    std::thread::spawn(move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let connection = zbus::blocking::Connection::session()?;

            let count = i64::from(visible);
            let mut props = std::collections::HashMap::new();
            props.insert("count", zbus::zvariant::Value::from(count));
            props.insert("count-visible", zbus::zvariant::Value::from(visible));

            connection.emit_signal(
                None::<zbus::names::BusName>,
                "/",
                "com.canonical.Unity.LauncherEntry",
                "Update",
                &(
                    format!("application://{}", versi_platform::DESKTOP_ENTRY_FILENAME),
                    props,
                ),
            )?;

            Ok(())
        })();

        if let Err(e) = result {
            debug!("Failed to set update badge: {e}");
        }
    });
}

pub(crate) fn set_dock_visible(_visible: bool) {}

pub(crate) fn is_wayland() -> bool {
    is_wayland_with(|name| std::env::var(name))
}

fn is_wayland_with<F>(get_var: F) -> bool
where
    F: for<'a> Fn(&'a str) -> Result<String, std::env::VarError>,
{
    get_var("XDG_SESSION_TYPE").map_or_else(
        |_| get_var("WAYLAND_DISPLAY").is_ok(),
        |value| value == "wayland",
    )
}

pub(crate) fn set_launch_at_login(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let autostart_dir = dirs::config_dir()
        .ok_or("could not determine config directory")?
        .join("autostart");
    let desktop_path = autostart_dir.join(versi_platform::DESKTOP_ENTRY_FILENAME);

    if !enable {
        if desktop_path.exists() {
            fs::remove_file(&desktop_path)?;
        }
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Versi\n\
         Exec={}\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display()
    );

    fs::create_dir_all(&autostart_dir)?;
    fs::write(&desktop_path, entry)?;
    Ok(())
}

pub(crate) fn reveal_in_file_manager(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::is_wayland_with;

    #[test]
    fn xdg_session_wayland_is_detected() {
        let result = is_wayland_with(|name| match name {
            "XDG_SESSION_TYPE" => Ok("wayland".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert!(result);
    }

    #[test]
    fn xdg_session_non_wayland_returns_false_without_fallback() {
        let result = is_wayland_with(|name| match name {
            "XDG_SESSION_TYPE" => Ok("x11".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert!(!result);
    }

    #[test]
    fn falls_back_to_wayland_display_when_session_type_missing() {
        let result = is_wayland_with(|name| match name {
            "XDG_SESSION_TYPE" => Err(std::env::VarError::NotPresent),
            "WAYLAND_DISPLAY" => Ok("wayland-0".to_string()),
            _ => Err(std::env::VarError::NotPresent),
        });

        assert!(result);
    }

    #[test]
    fn returns_false_when_no_wayland_variables_exist() {
        let result = is_wayland_with(|_| Err(std::env::VarError::NotPresent));

        assert!(!result);
    }
}
