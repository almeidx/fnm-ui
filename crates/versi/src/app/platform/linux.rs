pub(crate) fn set_update_badge(visible: bool) {
    use log::debug;

    std::thread::spawn(move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let connection = zbus::blocking::Connection::session()?;

            let count: i64 = if visible { 1 } else { 0 };
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
            debug!("Failed to set update badge: {}", e);
        }
    });
}

pub(crate) fn set_dock_visible(_visible: bool) {}

pub(crate) fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v == "wayland")
        .unwrap_or_else(|_| std::env::var("WAYLAND_DISPLAY").is_ok())
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
