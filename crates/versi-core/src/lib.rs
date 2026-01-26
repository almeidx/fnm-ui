mod backend;
mod detection;
mod error;
mod progress;
mod schedule;
mod update;
mod version;

pub mod commands;
pub use commands::HideWindow;

pub use backend::{Environment, FnmBackend};
pub use detection::{FnmDetection, detect_fnm, detect_fnm_dir, install_fnm};
pub use error::FnmError;
pub use progress::parse_progress_line;
pub use schedule::{ReleaseSchedule, fetch_release_schedule};
pub use update::{AppUpdate, FnmUpdate, check_for_fnm_update, check_for_update};
pub use version::{parse_installed_versions, parse_remote_versions};

pub use versi_backend::{
    BackendError, BackendInfo, InstallPhase, InstallProgress, InstalledVersion,
    ManagerCapabilities, NodeVersion, RemoteVersion, ShellInitOptions, VersionGroup,
    VersionManager, VersionParseError,
};
