use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum LaunchAtLoginError {
    #[cfg(target_os = "macos")]
    #[error("could not determine home directory")]
    HomeDirectoryUnavailable,
    #[cfg(target_os = "linux")]
    #[error("could not determine config directory")]
    ConfigDirectoryUnavailable,
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[cfg(windows)]
    #[error("registry call {operation} failed with status {status}")]
    Registry {
        operation: &'static str,
        status: i32,
    },
}

impl LaunchAtLoginError {
    pub(crate) fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(super) use linux::*;
#[cfg(target_os = "macos")]
pub(super) use macos::*;
#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub(super) use unsupported::*;
#[cfg(windows)]
pub(super) use windows::*;
