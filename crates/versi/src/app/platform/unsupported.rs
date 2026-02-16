pub(crate) fn set_update_badge(_visible: bool) {}

pub(crate) fn set_dock_visible(_visible: bool) {}

pub(crate) fn is_wayland() -> bool {
    false
}

pub(crate) fn set_launch_at_login(_enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub(crate) fn reveal_in_file_manager(_path: &std::path::Path) {}
