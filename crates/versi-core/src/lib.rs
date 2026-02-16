#![allow(clippy::missing_errors_doc)]

pub mod auto_update;
pub mod commands;
mod metadata;
mod schedule;
mod update;

pub use commands::HideWindow;
pub use metadata::{VersionMeta, fetch_version_metadata};
pub use schedule::{ReleaseSchedule, fetch_release_schedule};
pub use update::{AppUpdate, GitHubRelease, check_for_update, is_newer_version};
