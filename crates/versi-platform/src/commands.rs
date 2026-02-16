#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub trait HideWindow {
    fn hide_window(&mut self) -> &mut Self;
}

impl HideWindow for tokio::process::Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}

impl HideWindow for std::process::Command {
    #[cfg(windows)]
    fn hide_window(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(windows))]
    fn hide_window(&mut self) -> &mut Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::HideWindow;

    #[test]
    fn std_command_hide_window_is_chainable() {
        let mut cmd = std::process::Command::new("echo");
        let before = &mut cmd as *mut std::process::Command;
        let after = cmd.hide_window() as *mut std::process::Command;
        assert_eq!(before, after);
    }

    #[test]
    fn tokio_command_hide_window_is_chainable() {
        let mut cmd = tokio::process::Command::new("echo");
        let before = &mut cmd as *mut tokio::process::Command;
        let after = cmd.hide_window() as *mut tokio::process::Command;
        assert_eq!(before, after);
    }
}
