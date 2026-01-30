mod backend;
mod detection;
mod error;
mod progress;
mod provider;
mod schedule;
mod update;
mod version;

pub mod commands;
pub use commands::HideWindow;

pub use backend::{Environment, FnmBackend};
pub use error::FnmError;
pub use progress::parse_progress_line;
pub use provider::FnmProvider;
pub use schedule::{ReleaseSchedule, fetch_release_schedule};
pub use update::{AppUpdate, check_for_update};
pub use version::{parse_installed_versions, parse_remote_versions};

pub use versi_backend::{
    BackendDetection, BackendError, BackendInfo, BackendProvider, BackendUpdate, InstallPhase,
    InstallProgress, InstalledVersion, ManagerCapabilities, NodeVersion, RemoteVersion,
    ShellInitOptions, VersionGroup, VersionManager, VersionParseError,
};
