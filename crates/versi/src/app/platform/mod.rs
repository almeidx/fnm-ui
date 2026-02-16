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
