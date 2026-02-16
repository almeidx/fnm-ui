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
mod metadata;
mod schedule;
mod update;

/// Extension trait that normalizes "hide window" behavior on supported command
/// types.
pub use commands::HideWindow;
/// Release metadata model and fetch helper.
pub use metadata::{VersionMeta, fetch_version_metadata};
/// Node release schedule model and fetch helper.
pub use schedule::{ReleaseSchedule, fetch_release_schedule};
/// App update model, GitHub release mapping, and version comparison helpers.
pub use update::{AppUpdate, GitHubRelease, check_for_update, is_newer_version};
