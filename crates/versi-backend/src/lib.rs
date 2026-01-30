mod error;
mod traits;
mod types;

pub use error::BackendError;
pub use traits::{
    BackendDetection, BackendInfo, BackendProvider, BackendUpdate, ManagerCapabilities,
    ShellInitOptions, VersionManager,
};
pub use types::{
    InstallPhase, InstallProgress, InstalledVersion, NodeVersion, RemoteVersion, VersionGroup,
    VersionParseError,
};
