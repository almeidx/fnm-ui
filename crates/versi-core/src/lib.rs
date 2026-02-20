//! Core cross-crate utilities for Versi.
//!
//! This crate provides reusable logic that is independent of the UI and
//! concrete backend implementations:
//! - Release schedule loading and querying.
//! - Version metadata fetching.
//! - App update discovery and update payload types.
//! - Small platform command helpers (for example window-hiding adapters).

pub mod auto_update;
pub mod commands;
mod install_script;
mod metadata;
mod schedule;
mod update;

/// Extension trait that normalizes "hide window" behavior on supported command
/// types.
pub use commands::HideWindow;
/// Installer script download helper with retry/verification policy.
pub use install_script::{InstallScriptError, download_install_script_verified};
/// Release metadata model and fetch helper.
pub use metadata::{MetadataError, VersionMeta, fetch_version_metadata};
/// Node release schedule model and fetch helper.
pub use schedule::{ReleaseSchedule, ScheduleError, fetch_release_schedule};
/// App update model, GitHub release mapping, and version comparison helpers.
pub use update::{AppUpdate, GitHubRelease, UpdateError, check_for_update, is_newer_version};
